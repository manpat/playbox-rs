use crate::prelude::*;
use model::{World, GlobalVertexId, GlobalWallId};
use super::{Item, Operation, State, Context, EditorWorldEditCmd};

#[derive(Copy, Clone)]
enum ViewportItemShape {
	Vertex(Vec2),
	Line(Vec2, Vec2),
}

impl ViewportItemShape {
	fn distance_to(&self, target_pos: Vec2) -> f32 {
		match self {
			&ViewportItemShape::Vertex(v) => (target_pos - v).length(),

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
		}
	}
}

struct ViewportItem {
	shape: ViewportItemShape,
	item: Item,
	color: Color,
	transform: Mat2x3,
	interactive: bool,
}


#[derive(Clone, Debug)]
struct ViewportState {
	zoom: f32,
	camera_pivot: Vec2,
}


pub struct Viewport<'c> {
	painter: egui::Painter,
	response: egui::Response,

	editor_state: &'c mut State,
	viewport_state: ViewportState,
	viewport_metrics: ViewportMetrics,

	world: &'c World,
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
			});

		let viewport_metrics = ViewportMetrics::new(response.rect, &viewport_state);

		Self {
			painter,
			response,

			editor_state: &mut context.state,

			viewport_state,
			viewport_metrics,

			world: &context.model.world,
			message_bus: context.message_bus,
			items: Vec::new(),
		}
	}

	pub fn add_room(&mut self, room_index: usize, transform: Mat2x3) {
		let room = &self.world.rooms[room_index];
		let num_walls = room.walls.len();

		// Add vertices
		for (vertex_index, vertex) in room.wall_vertices.iter().enumerate() {
			self.items.push(ViewportItem {
				shape: ViewportItemShape::Vertex(transform * *vertex),
				item: Item::Vertex(GlobalVertexId {room_index, vertex_index}),
				color: Color::grey(0.5),
				transform,
				interactive: true,
			});
		}

		// Pick walls
		for wall_index in 0..num_walls {
			let (start, end) = room.wall_vertices(wall_index);

			self.items.push(ViewportItem {
				shape: ViewportItemShape::Line(transform * start, transform * end),
				item: Item::Wall(GlobalWallId {room_index, wall_index}),
				color: room.walls[wall_index].color,
				transform,
				interactive: true,
			});
		}

		// Pick room
		let room_center = room.wall_vertices.iter().sum::<Vec2>() / num_walls as f32;
		self.items.push(ViewportItem {
			shape: ViewportItemShape::Vertex(transform * room_center),
			item: Item::Room(room_index),
			color: Color::grey(0.5),
			transform,
			interactive: true,
		});
	}

	pub fn add_room_connections(&mut self, room_index: usize, transform: Mat2x3) {
		let room = &self.world.rooms[room_index];
		let num_walls = room.walls.len();

		for wall_index in 0..num_walls {
			let src_wall_id = GlobalWallId{room_index, wall_index};

			let Some(tgt_wall_id) = self.world.wall_target(src_wall_id) else {
				continue
			};

			let (src_start, src_end) = self.world.wall_vertices(src_wall_id);
			let (tgt_start, tgt_end) = self.world.wall_vertices(tgt_wall_id);

			let src_wall_length = (src_start - src_end).length();
			let tgt_wall_length = (tgt_start - tgt_end).length();

			let src_dir = (src_end - src_start).normalize();
			let src_center = (src_start + src_end) / 2.0;

			let apperture_half_size = src_wall_length.min(tgt_wall_length) / 2.0;
			let apperture_offset = src_dir.perp() * 0.25;

			let apperture_center = src_center + apperture_offset;
			let apperture_start = apperture_center - src_dir * apperture_half_size;
			let apperture_end = apperture_center + src_dir * apperture_half_size;

			self.items.push(ViewportItem {
				shape: ViewportItemShape::Line(transform * apperture_start, transform * apperture_end),
				item: Item::Wall(tgt_wall_id),
				color: Color::rgb(1.0, 0.6, 0.3),
				transform,
				interactive: false,
			});
		}
	}

	pub fn build(mut self) -> egui::Response {
		self.paint_background();

		self.handle_camera();

		// Figure out what is hovered
		if let Some(hover_pos) = self.response.hover_pos() {
			self.editor_state.hovered = None;
			self.handle_hover(self.viewport_metrics.widget_to_world_position(hover_pos));
		}

		if let Some(Operation::Drag{..}) = self.editor_state.current_operation {
			self.response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
		} else if self.editor_state.hovered.is_some() {
			self.response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
		}

		self.handle_item_interaction();

		self.handle_operation();

		self.draw_items();

		self.response.ctx.data_mut(|data| data.insert_temp(self.response.id, self.viewport_state));

		self.response
	}
}


impl Viewport<'_> {
	fn paint_background(&self) {
		let rect = self.response.rect;
		let center = self.response.rect.center();

		self.painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
		self.painter.hline(rect.x_range(), center.y, (1.0, egui::Color32::DARK_GRAY));
		self.painter.vline(center.x, rect.y_range(), (1.0, egui::Color32::DARK_GRAY));
	}

	fn handle_hover(&mut self, hover_pos_world: Vec2) {
		let mut min_distance = 0.3;

		for &ViewportItem {shape, item, transform, interactive, ..} in self.items.iter() {
			if !interactive {
				continue
			}

			let distance = shape.distance_to(hover_pos_world);
			if distance < min_distance {
				self.editor_state.hovered = Some(item);
				self.editor_state.hovered_transform = Some(transform); // TODO(pat.m): this maybe shouldn't be stored in editor state?
				min_distance = distance;
			}
		}
	}

	fn handle_camera(&mut self) {
		// Pan
		if self.response.dragged_by(egui::PointerButton::Middle) {
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

	fn handle_item_interaction(&mut self) {
		if self.response.is_pointer_button_down_on() && self.editor_state.hovered.is_some() && self.editor_state.current_operation.is_none() {
			self.editor_state.interaction_target = self.editor_state.hovered;
		}

		if self.response.drag_started_by(egui::PointerButton::Primary) {
			self.editor_state.current_operation = self.editor_state.interaction_target.zip(self.editor_state.hovered_transform)
				.map(|(item, room_to_world)| Operation::Drag{item, room_to_world});
		}

		if self.response.drag_stopped_by(egui::PointerButton::Primary) {
			self.editor_state.interaction_target = None;
			self.editor_state.current_operation = None;
		}

		if self.response.clicked() {
			self.editor_state.selection = self.editor_state.hovered;
		}

		if let Some(interaction_target) = self.editor_state.interaction_target {
			self.response.context_menu(|ui| {
				ui.set_min_width(200.0);

				match interaction_target {
					Item::Wall(wall_id) => {
						if ui.button("Add Vertex").clicked() {
							// let mouse_pos = self.response.interact_pointer_pos().unwrap();
							// let insert_pos = self.viewport_metrics.widget_to_world_position(mouse_pos);

							let (start, end) = self.world.wall_vertices(wall_id);
							let insert_pos = (start + end) / 2.0;

							self.message_bus.emit(EditorWorldEditCmd::SplitWall(wall_id, insert_pos));

							ui.close_menu();
						}

						if ui.button("Add Room").clicked() {
							ui.close_menu();
						}

						let wall_target = self.world.wall_target(wall_id);
						if wall_target.is_some() {
							if ui.button("Remove Connection").clicked() {
								self.message_bus.emit(EditorWorldEditCmd::DisconnectWall(wall_id));
								ui.close_menu();
							}
						} else {
							if ui.button("Add Connection").clicked() {
								ui.close_menu();
							}
						}
					}

					Item::Room(room_index) => {
						if ui.button("Remove Connections").clicked() {
							self.message_bus.emit(EditorWorldEditCmd::DisconnectRoom(room_index));
							ui.close_menu();
						}

						if ui.button("Delete Room").clicked() {
							self.message_bus.emit(EditorWorldEditCmd::RemoveRoom(room_index));
							ui.close_menu();
						}
					}

					Item::Vertex(_) => {
						if ui.button("Delete Vertex").clicked() {
							ui.close_menu();
						}
					}
				}
			});
		}

		if let Some(selected_item) = self.editor_state.selection {
			self.editor_state.focused_room_index = selected_item.room_index();
		}
	}

	fn draw_items(&self) {
		for &ViewportItem{item, shape, color, ..} in self.items.iter() {
			let item_hovered = self.editor_state.hovered == Some(item);
			let color = color.to_egui_rgba();

			match shape {
				ViewportItemShape::Vertex(vertex) => {
					if let Item::Room(room_index) = item {
						let vertex_px = self.viewport_metrics.world_to_widget_position(vertex);

						self.painter.text(
							vertex_px,
							egui::Align2::CENTER_CENTER,
							format!("#{room_index}"),
							egui::FontId::proportional(12.0),
							if item_hovered {
								egui::Color32::WHITE
							} else {
								egui::Color32::GRAY
							}
						);

					} else {
						let vertex_px = self.viewport_metrics.world_to_widget_position(vertex);
						let rect = egui::Rect::from_center_size(vertex_px, egui::vec2(12.0, 12.0));

						if item_hovered {
							self.painter.rect_filled(rect, 0.0, color);
						} else {
							self.painter.rect_stroke(rect, 0.0, (1.0, color));
						}
					}
				}

				ViewportItemShape::Line(start, end) => {
					let stroke_thickness = match item_hovered {
						false => 1.0,
						true => 4.0,
					};

					let start = self.viewport_metrics.world_to_widget_position(start);
					let end = self.viewport_metrics.world_to_widget_position(end);

					self.painter.line_segment([start, end], (stroke_thickness, color));
				}
			}
		}
	}

	fn handle_operation(&mut self) {
		match self.editor_state.current_operation {
			Some(Operation::Drag{item, room_to_world}) => {
				let world_delta = self.viewport_metrics.widget_to_world_delta(self.response.drag_delta());
				let room_delta = room_to_world.inverse() * world_delta.extend(0.0);

				self.message_bus.emit(EditorWorldEditCmd::TranslateItem(item, room_delta));
				self.response.mark_changed();
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
}