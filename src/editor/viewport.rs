use crate::prelude::*;
use model::{World, ProcessedWorld, WallId, RoomId, Placement, Location};
use super::{Item, InnerState, Context, EditorWorldEditCmd};

#[derive(Clone, Debug)]
enum ViewportItemShape {
	Vertex(Vec2),
	Line(Vec2, Vec2),

	PlayerIndicator(Mat2x3),
	ObjectIndicator(Mat2x3),

	Text(String, Vec2),
}

impl ViewportItemShape {
	fn distance_to(&self, target_pos: Vec2) -> f32 {
		match self {
			&ViewportItemShape::Vertex(v) | &ViewportItemShape::Text(_, v) => (target_pos - v).length(),

			&ViewportItemShape::Line(start, end) => {
				let wall_diff = end - start;
				let wall_length = wall_diff.length();
				let wall_direction = wall_diff / wall_length;

				let delta = target_pos - start;

				let distance_along = wall_direction.dot(delta) / wall_length;
				let distance_across = wall_direction.wedge(delta).abs();

				let buffer = 0.2;

				if distance_along < 0.0 || distance_along > 1.0 {
					return f32::INFINITY;
				}

				let point_a = start + wall_direction * wall_length * buffer;
				let point_b = end - wall_direction * wall_length * buffer;

				let dist_a = (point_a - target_pos).length();
				let dist_b = (point_b - target_pos).length();

				let mut distance = f32::INFINITY;

				if distance_along >= buffer && distance_along <= 1.0 - buffer {
					distance = distance.min(distance_across);
				}

				distance.min(dist_a).min(dist_b)
			}

			&ViewportItemShape::PlayerIndicator(_) => unimplemented!(),

			&ViewportItemShape::ObjectIndicator(transform) => {
				let [vx, vy, pos] = transform.columns();
				let radius = vx.square_length().max(vy.square_length()).sqrt();

				((target_pos - pos).length() - radius).max(0.0)
			}
		}
	}
}

const WALL_CONNECTION_COLOR: Color = Color::rgb(1.0, 0.6, 0.3);



bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct ViewportItemFlags: u32 {
		const DRAGGABLE = 1 << 0;
		const CLICKABLE = 1 << 1;
		const HAS_CONTEXT_MENU = 1 << 2;
		const BASIC_INTERACTIONS = 0b111;

		// Can rooms themselves be dragged?
		const RECENTERABLE = 1 << 3;

		// Can be a target for wall connections?
		const CONNECTABLE = 1 << 4;

		const ALL_INTERACTIONS = 0b11111;

		const SHOW_DEBUG_LABELS = 1 << 10;
	}
}

#[derive(Debug)]
struct ViewportItem {
	shape: ViewportItemShape,
	item: Option<Item>,
	color: Color,
	room_to_world: Mat2x3,
	flags: ViewportItemFlags,
}


#[derive(Clone, Debug)]
struct ViewportState {
	zoom: f32,
	camera_pivot: Vec2,

	context_menu_target: Option<Item>,
	context_menu_target_interact_pos: Vec2,

	hovered_item_flags: ViewportItemFlags,
	hovered_item_transform: Mat2x3,
	hovered_item_hover_pos: Vec2,

	current_operation: Option<Operation>,
}


pub struct Viewport<'c> {
	painter: egui::Painter,
	response: egui::Response,

	editor_state: &'c mut InnerState,
	viewport_state: ViewportState,
	viewport_metrics: ViewportMetrics,

	tracked_location: Option<Location>,

	world: &'c World,
	processed_world: &'c ProcessedWorld,
	message_bus: &'c MessageBus,

	items: Vec<ViewportItem>,
}

impl<'c> Viewport<'c> {
	pub fn new<'w: 'c>(ui: &mut egui::Ui, context: &'c mut Context<'w>) -> Self {
		let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), ui.available_height()), egui::Sense::click_and_drag());
		let viewport_state = ui.ctx().data_mut(|data| data.get_temp(response.id))
			.unwrap_or_else(|| ViewportState {
				zoom: 4.0,
				camera_pivot: Vec2::zero(),

				context_menu_target: None,
				context_menu_target_interact_pos: Vec2::zero(),

				hovered_item_flags: ViewportItemFlags::empty(),
				hovered_item_transform: Mat2x3::identity(),
				hovered_item_hover_pos: Vec2::zero(),

				current_operation: None,
			});

		let mut tracked_location = None;

		if context.state.track_player {
			tracked_location = Some(context.source_player_placement.location());
		}

		let viewport_metrics = ViewportMetrics::new(response.rect, &viewport_state);

		Self {
			painter,
			response,

			editor_state: &mut context.state,

			viewport_state,
			viewport_metrics,

			tracked_location,

			world: &context.model.world,
			processed_world: &context.runtime_model.processed_world,
			message_bus: context.message_bus,

			items: Vec::new(),
		}
	}

	pub fn add_room(&mut self, room_id: RoomId, room_to_world: Mat2x3, flags: ViewportItemFlags) {
		let source_geometry = &self.world.geometry;
		let processed_geometry = self.processed_world.geometry();

		let interaction_flags = flags.intersection(ViewportItemFlags::BASIC_INTERACTIONS);

		let mut room_interaction_flags = interaction_flags;
		if !flags.contains(ViewportItemFlags::RECENTERABLE) {
			room_interaction_flags.remove(ViewportItemFlags::DRAGGABLE);
		}

		let wall_interaction_flags = interaction_flags | ViewportItemFlags::CONNECTABLE;
		let show_debug_labels = flags.contains(ViewportItemFlags::SHOW_DEBUG_LABELS);

		// Add vertices
		for vertex_id in source_geometry.room_vertices(room_id) {
			let position = source_geometry.vertices[vertex_id].position;

			self.items.push(ViewportItem {
				shape: ViewportItemShape::Vertex(room_to_world * position),
				item: Some(Item::Vertex(vertex_id)),
				color: Color::grey(0.5),
				room_to_world,
				flags: interaction_flags,
			});
		}

		// Add walls
		for processed_room_id in self.processed_world.to_processed_rooms(room_id) {
			for wall_id in processed_geometry.room_walls(processed_room_id) {
				let (start, end) = processed_geometry.wall_vertices(wall_id);
				let direction = (end - start).normalize();
				let label_pos = (start + end) / 2.0 + direction * 0.05 + direction.perp() * 0.1;

				// Check if this wall exists in the source data (and so can be modified),
				// or was generated.
				if wall_id.is_valid(source_geometry) {
					self.items.push(ViewportItem {
						shape: ViewportItemShape::Line(room_to_world * start, room_to_world * end),
						item: Some(Item::Wall(wall_id)),
						color: wall_id.get(source_geometry).color,
						room_to_world,
						flags: wall_interaction_flags,
					});

					if show_debug_labels {
						self.items.push(ViewportItem {
							shape: ViewportItemShape::Text(format!("{wall_id:?}"), room_to_world * label_pos),
							item: Some(Item::Wall(wall_id)),
							color: wall_id.get(source_geometry).color,
							room_to_world,
							flags: wall_interaction_flags,
						});
					}
				} else {
					let processed_color = Color::rgba(0.2, 0.1, 0.1, 0.5);

					self.items.push(ViewportItem {
						shape: ViewportItemShape::Line(room_to_world * start, room_to_world * end),
						item: None,
						color: processed_color,
						room_to_world,
						flags: ViewportItemFlags::empty(),
					});

					if show_debug_labels {
						self.items.push(ViewportItem {
							shape: ViewportItemShape::Text(format!("{wall_id:?}"), room_to_world * label_pos),
							item: None,
							color: processed_color.with_alpha(0.8),
							room_to_world,
							flags: ViewportItemFlags::empty(),
						});
					}
				}
			}
		}

		// Pick room
		let num_walls = source_geometry.room_walls(room_id).count();
		let room_center = source_geometry.room_vertices(room_id)
			.map(|vertex_id| source_geometry.vertices[vertex_id].position)
			.sum::<Vec2>() / num_walls as f32;

		self.items.push(ViewportItem {
			shape: ViewportItemShape::Vertex(room_to_world * room_center),
			item: Some(Item::Room(room_id)),
			color: Color::grey(0.5),
			room_to_world,
			flags: room_interaction_flags,
		});
	}

	pub fn add_room_connections(&mut self, room_id: RoomId, room_to_world: Mat2x3, flags: ViewportItemFlags) {
		let geometry = &self.world.geometry;

		// Connections are only clickable
		let interaction_flags = flags.intersection(ViewportItemFlags::CLICKABLE) & !ViewportItemFlags::CONNECTABLE;

		for src_wall_id in geometry.room_walls(room_id) {
			let Some(wall_info) = self.processed_world.wall_info(src_wall_id) else {
				continue
			};

			let Some(connection_info) = &wall_info.connection_info else {
				continue
			};

			let visual_separation = wall_info.normal * 0.05;
			let aperture_start = connection_info.aperture_start + visual_separation;
			let aperture_end = connection_info.aperture_end + visual_separation;

			self.items.push(ViewportItem {
				shape: ViewportItemShape::Line(room_to_world * aperture_start, room_to_world * aperture_end),
				item: Some(Item::Wall(connection_info.target_wall)),
				color: WALL_CONNECTION_COLOR,
				room_to_world,
				flags: interaction_flags,
			});
		}
	}

	pub fn add_player_indicator(&mut self, placement: Placement, item: impl Into<Option<Item>>, color: impl Into<Color>, flags: ViewportItemFlags) {
		let transforms = self.items.iter()
			.filter(|vpitem| vpitem.item == Some(Item::Room(placement.room_id)))
			.map(|vpitem| vpitem.room_to_world)
			.collect::<Vec<_>>();

		let base_player_transform = Mat2x3::scale_rotate_translate(model::PLAYER_RADIUS, placement.yaw, placement.position);
		let item = item.into();
		let color = color.into();

		for room_to_world in transforms {
			self.items.push(ViewportItem {
				shape: ViewportItemShape::PlayerIndicator(room_to_world * base_player_transform),
				item,
				color,
				room_to_world,
				flags: flags,
			});
		}
	}

	pub fn add_object(&mut self, placement: Placement, item: impl Into<Option<Item>>, color: impl Into<Color>, flags: ViewportItemFlags) {
		let transforms = self.items.iter()
			.filter(|vpitem| vpitem.item == Some(Item::Room(placement.room_id)))
			.map(|vpitem| vpitem.room_to_world)
			.collect::<Vec<_>>();

		// TODO(pat.m): this should be based on the actual object
		let radius = 0.1;
		let base_transform = Mat2x3::scale_rotate_translate(radius, placement.yaw, placement.position);
		let item = item.into();
		let color = color.into();

		for room_to_world in transforms {
			self.items.push(ViewportItem {
				shape: ViewportItemShape::ObjectIndicator(room_to_world * base_transform),
				item,
				color,
				room_to_world,
				flags: flags,
			});
		}
	}

	pub fn build(mut self) -> egui::Response {
		self.paint_background();

		self.handle_camera();

		// Figure out what is hovered
		if let Some(hover_pos) = self.response.hover_pos() {
			self.handle_hover(self.viewport_metrics.widget_to_world_position(hover_pos));
		}

		self.handle_item_mouse_interaction();

		self.handle_operation();

		self.show_context_menu();

		if let Some(selected_item) = self.editor_state.selection {
			self.editor_state.focused_room_id = Some(selected_item.room_id(self.world));
		}

		self.draw_items();

		if let Some(operation) = &self.viewport_state.current_operation {
			self.draw_operation(operation);
		}

		// egui::Window::new("Viewport State")
		// 	.id(self.response.id.with("state window"))
		// 	.show(&self.response.ctx, |ui| {
		// 		ui.label(format!("{:#?}", self.viewport_state));
		// 	});

		if self.response.hovered() {
			self.set_cursor_state();
		}

		self.response.ctx.data_mut(|data| data.insert_temp(self.response.id, self.viewport_state));

		self.response
	}
}


impl Viewport<'_> {
	fn paint_background(&self) {
		let rect = self.response.rect;
		let center = self.response.rect.center();

		// self.painter.rect_filled(rect, 0.0, egui::Color32::BLACK.gamma_multiply(0.5));
		self.painter.hline(rect.x_range(), center.y, (1.0, egui::Color32::DARK_GRAY));
		self.painter.vline(center.x, rect.y_range(), (1.0, egui::Color32::DARK_GRAY));
	}

	fn handle_hover(&mut self, hover_pos_world: Vec2) {
		self.editor_state.hovered = None;

		let mut min_distance = 0.3;

		for ViewportItem {shape, item, room_to_world, flags, ..} in self.items.iter() {
			if !flags.intersects(ViewportItemFlags::BASIC_INTERACTIONS) {
				continue
			}

			let distance = shape.distance_to(hover_pos_world);
			if distance < min_distance {
				self.editor_state.hovered = *item;
				self.viewport_state.hovered_item_transform = *room_to_world;
				self.viewport_state.hovered_item_flags = *flags;
				self.viewport_state.hovered_item_hover_pos = room_to_world.inverse() * hover_pos_world;
				min_distance = distance;
			}
		}
	}

	fn handle_camera(&mut self) {
		// Pan to tracked location
		if let Some(Location{room_id, position}) = self.tracked_location
			&& let Some(vpitem) = self.items.iter()
				.find(|vpitem| vpitem.item == Some(Item::Room(room_id)))
		{
			self.viewport_state.camera_pivot = vpitem.room_to_world * position;
			self.viewport_metrics.update(&self.viewport_state);
		}

		// Pan
		if self.tracked_location.is_none() && self.response.dragged_by(egui::PointerButton::Middle) {
			self.viewport_state.camera_pivot -= self.viewport_metrics.widget_to_world_delta(self.response.drag_delta());
			self.viewport_metrics.update(&self.viewport_state);
		}

		// Zoom
		if let Some(hover_pos) = self.response.hover_pos() {
			let scroll_delta = self.response.ctx.input_mut(|input| std::mem::take(&mut input.smooth_scroll_delta.y));
			let hover_world_pre = self.viewport_metrics.widget_to_world_position(hover_pos);

			self.viewport_state.zoom *= (-scroll_delta / 100.0).exp2();
			self.viewport_state.zoom = self.viewport_state.zoom.clamp(1.0/2.0, 128.0);

			self.viewport_metrics.update(&self.viewport_state);
			let hover_world_post = self.viewport_metrics.widget_to_world_position(hover_pos);

			self.viewport_state.camera_pivot -= hover_world_post - hover_world_pre;
			self.viewport_metrics.update(&self.viewport_state);
		}
	}

	fn handle_item_mouse_interaction(&mut self) {
		if !self.response.hovered() {
			return
		}

		let are_clicks_consumed = self.viewport_state.current_operation.as_ref().map_or(false, Operation::consumes_clicks);
		if are_clicks_consumed {
			return
		}

		let is_hovered_draggable = self.viewport_state.hovered_item_flags.contains(ViewportItemFlags::DRAGGABLE);
		let is_hovered_clickable = self.viewport_state.hovered_item_flags.contains(ViewportItemFlags::CLICKABLE);
		let is_hovered_has_context_menu = self.viewport_state.hovered_item_flags.contains(ViewportItemFlags::HAS_CONTEXT_MENU);

		let is_primary_pressed = self.response.ctx.input(|input| input.pointer.primary_pressed());

		if let Some(item) = self.editor_state.hovered
			&& is_primary_pressed
			&& is_hovered_draggable
		{
			self.viewport_state.current_operation = Some(Operation::Drag {
				item,
				room_to_world: self.viewport_state.hovered_item_transform,
				click_to_confirm: false,
			});
		}

		if is_hovered_clickable && self.response.clicked() {
			self.editor_state.selection = self.editor_state.hovered;
		}

		if self.response.secondary_clicked() {
			if is_hovered_has_context_menu {
				self.viewport_state.context_menu_target = self.editor_state.hovered;
				self.viewport_state.context_menu_target_interact_pos = self.viewport_state.hovered_item_hover_pos;
			} else {
				self.viewport_state.context_menu_target = None;
			}
		}
	}

	fn show_context_menu(&mut self) {
		let Some(item) = self.viewport_state.context_menu_target else {
			return;
		};

		self.response.context_menu(|ui| {
			ui.set_min_width(200.0);

			match item {
				Item::Wall(wall_id) => {
					let wall_target = self.world.geometry.wall_target(wall_id);
					if wall_target.is_some() {
						if ui.button("Reconnect").clicked() {
							self.viewport_state.current_operation = Some(Operation::ConnectWall{
								source_wall: wall_id,
								room_to_world: self.viewport_state.hovered_item_transform,
							});
							ui.close_menu();
						}

						if ui.button("Disconnect").clicked() {
							self.message_bus.emit(EditorWorldEditCmd::DisconnectWall(wall_id));
							ui.close_menu();
						}
					} else {
						if ui.button("Connect").clicked() {
							self.viewport_state.current_operation = Some(Operation::ConnectWall{
								source_wall: wall_id,
								room_to_world: self.viewport_state.hovered_item_transform,
							});
							ui.close_menu();
						}
					}

					ui.separator();

					if ui.button("Split Wall").clicked() {
						let interact_pos = self.viewport_state.context_menu_target_interact_pos;
						let (start, end) = self.world.geometry.wall_vertices(wall_id);
						let length = (end - start).length();
						let direction = (end - start) / length;

						let distance_along = (interact_pos - start).dot(direction);
						let insert_pos = start + direction * distance_along.clamp(0.01, length-0.01);

						self.message_bus.emit(EditorWorldEditCmd::SplitWall(wall_id, insert_pos));

						ui.close_menu();
					}

					if ui.button("Split Room").clicked() {
						self.viewport_state.current_operation = Some(Operation::SplitRoom {
							source_wall: wall_id,
							room_to_world: self.viewport_state.hovered_item_transform,
						});

						ui.close_menu();
					}

					if ui.button("Extrude").clicked() {
						let geometry = &self.world.geometry;

						self.message_bus.emit(EditorWorldEditCmd::SplitVertex(wall_id.vertex(geometry)));
						self.message_bus.emit(EditorWorldEditCmd::SplitVertex(wall_id.next_vertex(geometry)));

						self.viewport_state.current_operation = Some(Operation::Drag{
							item: Item::Wall(wall_id),
							room_to_world: self.viewport_state.hovered_item_transform,
							click_to_confirm: true,
						});
						ui.close_menu();
					}

					if ui.button("Add Room").clicked() {
						todo!();

						// let wall_length = self.world.geometry.wall_length(wall_id);
						// self.message_bus.emit(EditorWorldEditCmd::AddRoom {
						// 	room: RoomDef::new_square(wall_length),
						// 	connection: Some((todo!(), wall_id)),
						// });

						// ui.close_menu();
					}
				}

				Item::Room(room_id) => {
					if ui.button("Disconnect All").clicked() {
						self.message_bus.emit(EditorWorldEditCmd::DisconnectRoom(room_id));
						ui.close_menu();
					}

					if ui.button("Delete").clicked() {
						self.message_bus.emit(EditorWorldEditCmd::RemoveRoom(room_id));
						ui.close_menu();
					}
				}

				Item::Vertex(vertex_id) => {
					if ui.button("Split Room").clicked() {
						// TODO(pat.m): require that vertex is unique!
						self.viewport_state.current_operation = Some(Operation::SplitRoom {
							source_wall: vertex_id.wall(&self.world.geometry),
							room_to_world: self.viewport_state.hovered_item_transform,
						});

						ui.close_menu();
					}

					if ui.button("Bevel").clicked() {
						let geometry = &self.world.geometry;

						// TODO(pat.m): assert that vertex has only single outgoing wall
						let outgoing_wall = vertex_id.wall(geometry);
						let incoming_wall = outgoing_wall.prev_wall(geometry);

						let incoming_start = incoming_wall.vertex(geometry).position(geometry);
						let (original_vertex, outgoing_end) = geometry.wall_vertices(outgoing_wall);

						let incoming_direction = incoming_start - original_vertex;
						let outgoing_direction = outgoing_end - original_vertex;

						// Bevel to half way along the shortest wall
						let bevel_dist = incoming_direction.length().min(outgoing_direction.length()) / 2.0;

						let start_vertex = original_vertex + incoming_direction.normalize() * bevel_dist;
						let end_vertex_delta = outgoing_direction.normalize() * bevel_dist;

						// Translate the original vertex along the _incoming_ wall
						self.message_bus.emit(EditorWorldEditCmd::TranslateItem(Item::Vertex(vertex_id), end_vertex_delta));

						// Split the _outgoing_ wall and place the new vertex at the end pos.
						self.message_bus.emit(EditorWorldEditCmd::SplitWall(incoming_wall, start_vertex));

						ui.close_menu();
					}

					ui.separator();

					if ui.button("Delete Vertex").clicked() {
						self.message_bus.emit(EditorWorldEditCmd::DeleteVertex(vertex_id));
						ui.close_menu();
					}
				}

				_ => todo!(),
			}
		});
	}

	fn set_cursor_state(&self) {
		if let Some(operation) = self.viewport_state.current_operation {
			// If we have an operation active - _only_ use whatever cursor the operation requests
			if let Some(cursor) = self.operation_cursor_icon(&operation) {
				self.response.ctx.set_cursor_icon(cursor);
			}

		} else if self.editor_state.hovered.is_some() && self.viewport_state.hovered_item_flags.contains(ViewportItemFlags::DRAGGABLE) {
			self.response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
		}
	}

	fn draw_items(&self) {
		for ViewportItem{item, shape, color, ..} in self.items.iter() {
			let item_hovered = self.editor_state.hovered == *item && item.is_some();
			let item_selected = self.editor_state.selection == *item && item.is_some();
			let color = color.to_egui_rgba();

			match shape {
				&ViewportItemShape::Vertex(vertex) => {
					if let Some(Item::Room(room_id)) = item {
						let vertex_px = self.viewport_metrics.world_to_widget_position(vertex);

						self.painter.text(
							vertex_px,
							egui::Align2::CENTER_CENTER,
							format!("{room_id:?}"),
							egui::FontId::proportional(12.0),
							if item_hovered || item_selected {
								egui::Color32::WHITE
							} else {
								egui::Color32::GRAY
							}
						);

					} else {
						let vertex_px = self.viewport_metrics.world_to_widget_position(vertex);
						let size_widget = self.viewport_metrics.world_to_widget_scalar(0.2).min(12.0);
						let rect = egui::Rect::from_center_size(vertex_px, egui::vec2(size_widget, size_widget));

						if item_hovered || item_selected {
							self.painter.rect_filled(rect, 0.0, color);
						} else {
							self.painter.rect_stroke(rect, 0.0, (1.0, color));
						}
					}
				}

				&ViewportItemShape::Line(start, end) => {
					let stroke_thickness = match item_hovered || item_selected {
						false => 1.0,
						true => 4.0,
					};

					let start = self.viewport_metrics.world_to_widget_position(start);
					let end = self.viewport_metrics.world_to_widget_position(end);

					self.painter.line_segment([start, end], (stroke_thickness, color));
				}

				ViewportItemShape::Text(text, position) => {
					let position_px = self.viewport_metrics.world_to_widget_position(*position);

					self.painter.text(
						position_px,
						egui::Align2::CENTER_CENTER,
						text,
						egui::FontId::proportional(12.0),
						color.into()
					);
				}

				&ViewportItemShape::PlayerIndicator(transform) => {
					let forward = -transform.column_y();
					let origin = transform.column_z();

					let center_widget = self.viewport_metrics.world_to_widget_position(origin);
					let forward_widget = self.viewport_metrics.world_to_widget_delta(forward);

					let point = center_widget + forward_widget * 3.0;

					self.painter.circle_stroke(center_widget, forward_widget.length(), (1.0, color));
					self.painter.line_segment([center_widget, point], (1.0, color));
				}

				&ViewportItemShape::ObjectIndicator(transform) => {
					let right = transform.column_x();
					let forward = -transform.column_y();
					let origin = transform.column_z();

					let center_widget = self.viewport_metrics.world_to_widget_position(origin);
					let forward_widget = self.viewport_metrics.world_to_widget_delta(forward);
					let right_widget = self.viewport_metrics.world_to_widget_delta(right);

					let points = [
						center_widget + ( forward_widget + right_widget),
						center_widget + ( forward_widget - right_widget),
						center_widget + (-forward_widget - right_widget),
						center_widget + (-forward_widget + right_widget),
						center_widget + ( forward_widget + right_widget),
					];

					// Draw box
					for i in 0..4 {
						self.painter.line_segment([points[i], points[i+1]], (1.0, color));
					}

					// Draw orientation
					self.painter.line_segment([center_widget, center_widget + right_widget / 2.0], (1.0, egui::Color32::LIGHT_RED));
					self.painter.line_segment([center_widget, center_widget + forward_widget / 2.0], (1.0, egui::Color32::LIGHT_GREEN));
				}
			}
		}
	}

	fn draw_operation(&self, operation: &Operation) {
		match operation {
			&Operation::ConnectWall{source_wall, room_to_world} => {
				let center_room = self.world.geometry.wall_center(source_wall);
				let center_widget = self.viewport_metrics.world_to_widget_position(room_to_world * center_room);

				let is_hovered_connectable = self.viewport_state.hovered_item_flags.contains(ViewportItemFlags::CONNECTABLE);

				match self.editor_state.hovered {
					Some(Item::Wall(wall_id)) if wall_id != source_wall && is_hovered_connectable => {
						let center_room = self.world.geometry.wall_center(wall_id);

						// Draw a line to each instance of the room in the viewport
						let transforms = self.items.iter()
							.filter(|vpitem| {
								vpitem.item == Some(Item::Wall(wall_id))
								&& vpitem.flags.contains(ViewportItemFlags::CONNECTABLE)
							})
							.map(|vpitem| vpitem.room_to_world);

						for transform in transforms {
							let target_widget = self.viewport_metrics.world_to_widget_position(transform * center_room);
							self.painter.arrow(center_widget, target_widget-center_widget, (1.0, WALL_CONNECTION_COLOR.to_egui_rgba()));
						}
					}

					_ => {
						let target_widget = self.response.ctx.input(|input| input.pointer.latest_pos())
							.unwrap_or(egui::pos2(0.0, 0.0));

						self.painter.arrow(center_widget, target_widget-center_widget, (1.0, WALL_CONNECTION_COLOR.to_egui_rgba()));
					}
				}
			}

			&Operation::SplitRoom{source_wall, room_to_world} => {
				let geometry = &self.world.geometry;
				let source_vertex_position = source_wall.vertex(geometry).position(geometry);
				let line_start = self.viewport_metrics.world_to_widget_position(room_to_world * source_vertex_position);

				match self.editor_state.hovered {
					Some(Item::Wall(target_wall)) /*if valid_connection(source_wall, target_wall)*/ => {
						let target_vertex_position = target_wall.next_vertex(geometry).position(geometry);
						let line_end = self.viewport_metrics.world_to_widget_position(room_to_world * target_vertex_position);

						self.painter.line_segment([line_start, line_end], (1.0, WALL_CONNECTION_COLOR.to_egui_rgba()));
					}

					Some(Item::Vertex(target_vertex)) /*if valid_connection(source_wall, target_wall)*/ => {
						let target_vertex_position = target_vertex.position(geometry);
						let line_end = self.viewport_metrics.world_to_widget_position(room_to_world * target_vertex_position);

						self.painter.line_segment([line_start, line_end], (1.0, WALL_CONNECTION_COLOR.to_egui_rgba()));
					}

					_ => {
						let line_end = self.response.ctx.input(|input| input.pointer.latest_pos())
							.unwrap_or(egui::pos2(0.0, 0.0));

						self.painter.line_segment([line_start, line_end], (1.0, WALL_CONNECTION_COLOR.to_egui_rgba()));
					}
				}
			}

			_ => {}
		}
	}

	fn operation_cursor_icon(&self, operation: &Operation) -> Option<egui::CursorIcon> {
		match operation {
			Operation::Drag{..} => Some(egui::CursorIcon::Grabbing),
			Operation::ConnectWall{source_wall, ..} => {
				let is_hovered_connectable = self.viewport_state.hovered_item_flags.contains(ViewportItemFlags::CONNECTABLE);

				if let Some(Item::Wall(target_wall)) = self.editor_state.hovered
					&& target_wall != *source_wall
					&& is_hovered_connectable
				{
					Some(egui::CursorIcon::PointingHand)
				}
				else {
					None
				}
			}

			Operation::SplitRoom{source_wall, ..} => {
				let geometry = &self.world.geometry;

				match self.editor_state.hovered {
					Some(Item::Wall(target_wall)) if target_wall != *source_wall => Some(egui::CursorIcon::PointingHand),
					Some(Item::Vertex(target_vertex)) if target_vertex.wall(geometry).prev_wall(geometry) != *source_wall => Some(egui::CursorIcon::PointingHand),
					_ => None
				}
			}
		}
	}

	fn handle_operation(&mut self) {
		match self.viewport_state.current_operation {
			Some(Operation::Drag{item, room_to_world, click_to_confirm}) => {
				let delta = match click_to_confirm {
					true => self.response.ctx.input(|input| input.pointer.delta()),
					false => self.response.drag_delta(),
				};

				let world_delta = self.viewport_metrics.widget_to_world_delta(delta);
				let room_delta = room_to_world.inverse() * world_delta.extend(0.0);

				self.message_bus.emit(EditorWorldEditCmd::TranslateItem(item, room_delta));
				self.response.mark_changed();

				// TODO(pat.m): do I actually need this? maybe just release is fine
				let confirmed = match click_to_confirm {
					true => self.response.clicked(),
					false => self.response.ctx.input(|input| input.pointer.primary_released()),
				};

				// TODO(pat.m): cancellation

				if confirmed {
					self.viewport_state.current_operation = None;
				}
			}

			Some(Operation::ConnectWall{source_wall, ..}) => {
				let is_primary_pressed = self.response.ctx.input(|input| input.pointer.primary_pressed());
				let is_secondary_pressed = self.response.ctx.input(|input| input.pointer.secondary_pressed());

				// If left clicked either commit if hovering a wall or cancel
				if is_primary_pressed {
					if let Some(Item::Wall(target_wall)) = self.editor_state.hovered
						&& target_wall != source_wall
						&& self.viewport_state.hovered_item_flags.contains(ViewportItemFlags::CONNECTABLE)
					{
						self.message_bus.emit(EditorWorldEditCmd::ConnectWall(source_wall, target_wall));
					}

					self.viewport_state.current_operation = None;
				}

				// Cancel if clicked outside the widget or right clicked
				if self.response.clicked_elsewhere() || is_secondary_pressed {
					self.viewport_state.current_operation = None;
				}
			}

			Some(Operation::SplitRoom{source_wall, ..}) => {
				let is_primary_pressed = self.response.ctx.input(|input| input.pointer.primary_pressed());
				let is_secondary_pressed = self.response.ctx.input(|input| input.pointer.secondary_pressed());

				// If left clicked either commit if hovering a wall or cancel
				if is_primary_pressed {
					match self.editor_state.hovered {
						Some(Item::Wall(target_wall)) => {
							// TODO(pat.m): if valid_split
							if target_wall != source_wall {
								self.message_bus.emit(EditorWorldEditCmd::SplitRoom(source_wall, target_wall));
							}
						}

						Some(Item::Vertex(target_vertex)) => {
							let geometry = &self.world.geometry;
							let target_wall = target_vertex.wall(geometry).prev_wall(geometry);

							// TODO(pat.m): if valid_split
							if target_wall != source_wall && target_wall.room(geometry) == source_wall.room(geometry) {
								self.message_bus.emit(EditorWorldEditCmd::SplitRoom(source_wall, target_wall));
							}
						}

						_ => {}
					}

					self.viewport_state.current_operation = None;
				}

				// Cancel if clicked outside the widget or right clicked
				if self.response.clicked_elsewhere() || is_secondary_pressed {
					self.viewport_state.current_operation = None;
				}
			}

			_ => {}
		}
	}
}




struct ViewportMetrics {
	widget_center: Vec2,
	widget_extent: Vec2,

	camera_pivot: Vec2,
	zoom: f32,
}


impl ViewportMetrics {
	fn new(rect: egui::Rect, viewport_state: &ViewportState) -> Self {
		let widget_center = Vec2::from_compatible(rect.center());
		let widget_extent = Vec2::from_compatible(rect.size() / 2.0);

		ViewportMetrics {
			widget_center,
			widget_extent,

			camera_pivot: viewport_state.camera_pivot,
			zoom: viewport_state.zoom,
		}
	}

	fn update(&mut self, viewport_state: &ViewportState) {
		self.zoom = viewport_state.zoom;
		self.camera_pivot = viewport_state.camera_pivot;
	}

	fn world_to_widget_scale_factor(&self) -> f32 {
		self.widget_extent.x / self.zoom
	}

	fn widget_to_world_position(&self, pos: egui::Pos2) -> Vec2 {
		let camera_pos = (Vec2::from_compatible(pos) - self.widget_center) / self.world_to_widget_scale_factor();
		camera_pos + self.camera_pivot
	}

	fn widget_to_world_delta(&self, pos: egui::Vec2) -> Vec2 {
		Vec2::from_compatible(pos) / self.world_to_widget_scale_factor()
	}

	fn world_to_widget_position(&self, pos: Vec2) -> egui::Pos2 {
		let widget_relative_pos = (pos - self.camera_pivot) * self.world_to_widget_scale_factor();
		(self.widget_center + widget_relative_pos).to_egui_pos2()
	}

	fn world_to_widget_delta(&self, pos: Vec2) -> egui::Vec2 {
		(pos * self.world_to_widget_scale_factor()).to_egui_vec2()
	}

	fn world_to_widget_scalar(&self, scalar: f32) -> f32 {
		scalar * self.world_to_widget_scale_factor()
	}
}



#[derive(Copy, Clone, Debug)]
enum Operation {
	Drag {
		item: Item,
		room_to_world: Mat2x3,
		click_to_confirm: bool,
	},

	ConnectWall {
		source_wall: WallId,
		room_to_world: Mat2x3,
	},

	SplitRoom {
		source_wall: WallId,
		room_to_world: Mat2x3,
	},
}

impl Operation {
	fn consumes_clicks(&self) -> bool {
		match self {
			Self::ConnectWall{..} | Self::SplitRoom{..} => true,
			Self::Drag{click_to_confirm, ..} => *click_to_confirm,
		}
	}
}