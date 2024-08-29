use crate::prelude::*;
use model::{World, GlobalVertexId, GlobalWallId};
use super::{Item, Operation, State, Context, EditorCmd};

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

		Self {
			painter,
			response,

			editor_state: &mut context.state,
			viewport_state,

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
			});
		}

		// Pick room
		let room_center = room.wall_vertices.iter().sum::<Vec2>() / num_walls as f32;
		self.items.push(ViewportItem {
			shape: ViewportItemShape::Vertex(transform * room_center),
			item: Item::Room(room_index),
			color: Color::grey(0.5),
			transform,
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
			});
		}
	}

	pub fn build(mut self) -> egui::Response {
		self.paint_background();

		self.handle_camera();

		// Figure out what is hovered (if no operations are happening)
		if self.editor_state.current_operation.is_none() && !self.response.context_menu_opened() {
			self.editor_state.hovered = None;

			if let Some(hover_pos) = self.response.hover_pos() {
				self.handle_hover(self.widget_to_world_position(hover_pos));
			}
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
	fn widget_to_world_position(&self, pos: egui::Pos2) -> Vec2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = self.viewport_state.zoom;
		let scale_factor = widget_extent / viewport_extent;
		let local_pos = (pos - rect.center()) / scale_factor;

		Vec2::from_compatible(local_pos) + self.viewport_state.camera_pivot
	}

	fn widget_to_world_delta(&self, pos: egui::Vec2) -> Vec2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = self.viewport_state.zoom;
		let scale_factor = widget_extent / viewport_extent;

		Vec2::from_compatible(pos / scale_factor)
	}

	fn world_to_widget_position(&self, pos: Vec2) -> egui::Pos2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = self.viewport_state.zoom;
		let scale_factor = widget_extent / viewport_extent;

		rect.center() + ((pos - self.viewport_state.camera_pivot) * scale_factor).to_egui_vec2()
	}

	fn world_to_widget_delta(&self, pos: Vec2) -> egui::Vec2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = self.viewport_state.zoom;
		let scale_factor = widget_extent / viewport_extent;

		(pos * scale_factor).to_egui_vec2()
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

		for &ViewportItem {shape, item, transform, ..} in self.items.iter() {
			let distance = shape.distance_to(hover_pos_world);
			if distance < min_distance {
				self.editor_state.hovered = Some(item);
				self.editor_state.hovered_transform = Some(transform);
				min_distance = distance;
			}
		}
	}

	fn handle_camera(&mut self) {
		if self.response.dragged_by(egui::PointerButton::Middle) {
			self.viewport_state.camera_pivot -= self.widget_to_world_delta(self.response.drag_delta());
		}

		if let Some(hover_pos) = self.response.hover_pos() {
			let scroll_delta = self.response.ctx.input_mut(|input| std::mem::take(&mut input.smooth_scroll_delta.y));
			let hover_world_pre = self.widget_to_world_position(hover_pos);

			let prev_zoom = self.viewport_state.zoom;
			self.viewport_state.zoom *= (-scroll_delta / 100.0).exp2();
			self.viewport_state.zoom = self.viewport_state.zoom.clamp(1.0/2.0, 128.0);

			let hover_world_post = self.widget_to_world_position(hover_pos);

			self.viewport_state.camera_pivot -= (hover_world_post - hover_world_pre);
		}
	}

	fn handle_item_interaction(&mut self) {
		if self.response.drag_started_by(egui::PointerButton::Primary) {
			self.editor_state.current_operation = self.editor_state.hovered.zip(self.editor_state.hovered_transform)
				.map(|(item, room_to_world)| Operation::Drag{item, room_to_world});
		}

		if self.response.clicked() {
			self.editor_state.selection = self.editor_state.hovered;
		}

		if self.response.drag_stopped_by(egui::PointerButton::Primary) {
			self.editor_state.current_operation = None;
		}

		if let Some(hovered_item) = self.editor_state.hovered {
			self.response.context_menu(|ui| {
				ui.set_min_width(200.0);

				match hovered_item {
					Item::Wall(wall_id) => {
						let wall_target = self.world.wall_target(wall_id);

						if ui.button("Add Vertex").clicked() {
							ui.close_menu();
						}

						if ui.button("Add Room").clicked() {
							ui.close_menu();
						}

						if wall_target.is_some() {
							if ui.button("Remove Connection").clicked() {
								ui.close_menu();
							}
						} else {
							if ui.button("Add Connection").clicked() {
								ui.close_menu();
							}
						}
					}

					Item::Room(_room_index) => {
						if ui.button("Delete Room").clicked() {
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
						let vertex_px = self.world_to_widget_position(vertex);

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
						let vertex_px = self.world_to_widget_position(vertex);
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

					let start = self.world_to_widget_position(start);
					let end = self.world_to_widget_position(end);

					self.painter.line_segment([start, end], (stroke_thickness, color));
				}
			}
		}
	}

	fn handle_operation(&mut self) {
		match self.editor_state.current_operation {
			Some(Operation::Drag{item, room_to_world}) => {
				let world_delta = self.widget_to_world_delta(self.response.drag_delta());
				let room_delta = room_to_world.inverse() * world_delta.extend(0.0);

				self.message_bus.emit(EditorCmd::TranslateItem(item, room_delta));
				self.response.mark_changed();
			}

			_ => {}
		}
	}
}