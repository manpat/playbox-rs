use crate::prelude::*;
use world::{World, WorldChangedEvent, GlobalVertexId, GlobalWallId};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Item {
	Vertex(GlobalVertexId),
	Wall(GlobalWallId),
	Room(usize),
}

impl Item {
	fn room_index(&self) -> usize {
		match *self {
			Item::Room(room_index) => room_index,
			Item::Vertex(GlobalVertexId{room_index, ..}) | Item::Wall(GlobalWallId{room_index, ..}) => room_index,
		}
	}
}

#[derive(Copy, Clone, Debug)]
enum Operation {
	Drag(Item),
}

#[derive(Copy, Clone, Default, Debug)]
struct State {
	hovered: Option<Item>,
	selection: Option<Item>,

	operation: Option<Operation>,
}

struct Context<'w> {
	state: State,
	world: &'w mut World,
	message_bus: &'w MessageBus,
}

pub fn draw_world_editor(ctx: &egui::Context, world: &mut World, message_bus: &MessageBus) {
	let mut changed = false;

	let mut context = Context {
		state: ctx.data(|map| map.get_temp(egui::Id::null()).unwrap_or_default()),
		world,
		message_bus,
	};

	egui::Window::new("World")
		.show(ctx, |ui| {
			changed |= ui.color_edit_button_rgb(context.world.fog_color.as_mut()).changed();
		});

	egui::Window::new("Viewport")
		.show(ctx, |ui| {
			draw_room_selector(ui, &mut context);

			let selected_room_index = context.state.selection.as_ref().map_or(0, Item::room_index);

			let mut viewport = Viewport::new(ui, &mut context);
			viewport.add_room_at(selected_room_index, Vec2::zero());
			// viewport.add_room_at(1, Vec2::from_x(4.0));

			changed |= viewport.ui(ui).changed();
		});

	egui::Window::new("Inspector")
		.show(ctx, |ui| {
			changed |= draw_item_inspector(ui, &mut context);
		});

	ctx.data_mut(move |map| map.insert_temp(egui::Id::null(), context.state));

	if changed {
		message_bus.emit(WorldChangedEvent);
	}
}


fn draw_room_selector(ui: &mut egui::Ui, Context{world, state, ..}: &mut Context) {
	let selected_room_index = state.selection.as_ref().map_or(0, Item::room_index);

	ui.horizontal(|ui| {
		for (room_index, _room) in world.rooms.iter().enumerate() {
			let selected = room_index == selected_room_index;
			if ui.selectable_label(selected, format!("{room_index}")).clicked() {
				state.selection = Some(Item::Room(room_index));
			}
		}
	});
}

fn draw_item_inspector(ui: &mut egui::Ui, ctx: &mut Context) -> bool {
	match ctx.state.selection {
		None => {
			ui.label("<select an item>");
			false
		}

		Some(Item::Vertex(vertex_id)) => {
			draw_room_inspector(ui, ctx, vertex_id.room_index)
		}

		Some(Item::Wall(wall_id)) => {
			let mut changed = false;
			changed |= draw_room_inspector(ui, ctx, wall_id.room_index);
			ui.separator();
			changed |= draw_wall_inspector(ui, ctx, wall_id);
			changed
		}

		Some(Item::Room(room_index)) => {
			draw_room_inspector(ui, ctx, room_index)
		}
	}

}

fn draw_room_inspector(ui: &mut egui::Ui, Context{world, ..}: &mut Context, room_index: usize) -> bool {
	let Some(room) = world.rooms.get_mut(room_index) else {
		return false
	};

	ui.label(format!("Room #{room_index}"));

	let mut changed = false;

	ui.horizontal(|ui| {
		ui.label("Ceiling");
		changed |= ui.color_edit_button_rgb(room.ceiling_color.as_mut()).changed();
	});

	ui.horizontal(|ui| {
		ui.label("Floor");
		changed |= ui.color_edit_button_rgb(room.floor_color.as_mut()).changed();
	});

	changed
}

fn draw_wall_inspector(ui: &mut egui::Ui, Context{world, ..}: &mut Context, GlobalWallId{room_index, wall_index}: GlobalWallId) -> bool {
	let Some(wall) = world.rooms.get_mut(room_index)
		.and_then(|room| room.walls.get_mut(wall_index))
	else {
		return false
	};

	ui.label(format!("Wall #{wall_index}"));

	let mut changed = false;

	ui.horizontal(|ui| {
		ui.label("Wall");
		changed |= ui.color_edit_button_rgb(wall.color.as_mut()).changed();
	});

	changed
}


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
}


struct Viewport<'c> {
	painter: egui::Painter,
	response: egui::Response,

	state: &'c mut State,
	world: &'c mut World,

	items: Vec<ViewportItem>,
}

impl<'c> Viewport<'c> {
	pub fn new<'w: 'c>(ui: &mut egui::Ui, context: &'c mut Context<'w>) -> Self {
		let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), ui.available_height()), egui::Sense::click_and_drag());

		Self {
			painter,
			response,
			world: context.world,
			state: &mut context.state,
			items: Vec::new(),
		}
	}

	pub fn add_room_at(&mut self, room_index: usize, offset: Vec2) {
		let room = &self.world.rooms[room_index];
		let num_walls = room.walls.len();

		// Add vertices
		for (vertex_index, vertex) in room.wall_vertices.iter().enumerate() {
			self.items.push(ViewportItem {
				shape: ViewportItemShape::Vertex(*vertex + offset),
				item: Item::Vertex(GlobalVertexId {room_index, vertex_index}),
				color: Color::grey(0.5),
			});
		}

		// Pick walls
		for wall_index in 0..num_walls {
			let (start, end) = room.wall_vertices(wall_index);

			self.items.push(ViewportItem {
				shape: ViewportItemShape::Line(start + offset, end + offset),
				item: Item::Wall(GlobalWallId {room_index, wall_index}),
				color: room.walls[wall_index].color,
			});
		}

		// Pick room
		let room_center = room.wall_vertices.iter().sum::<Vec2>() / num_walls as f32;
		self.items.push(ViewportItem {
			shape: ViewportItemShape::Vertex(room_center + offset),
			item: Item::Room(room_index),
			color: Color::grey(0.5),
		});
	}

	fn widget_to_world_pos(&self, pos: egui::Pos2) -> Vec2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = 4.0; // TODO(pat.m): should come from zoom
		let scale_factor = widget_extent / viewport_extent;
		let local_pos = (pos - rect.center()) / scale_factor;

		Vec2::from_compatible(local_pos)
	}

	fn widget_to_world_delta(&self, pos: egui::Vec2) -> Vec2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = 4.0; // TODO(pat.m): should come from zoom
		let scale_factor = widget_extent / viewport_extent;

		Vec2::from_compatible(pos / scale_factor)
	}

	fn world_to_widget_pos(&self, pos: Vec2) -> egui::Pos2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = 4.0; // TODO(pat.m): should come from zoom
		let scale_factor = widget_extent / viewport_extent;

		rect.center() + (pos * scale_factor).to_egui_vec2()
	}

	fn world_to_widget_delta(&self, pos: Vec2) -> egui::Vec2 {
		let rect = self.response.rect;

		let widget_extent = rect.size().x / 2.0;
		let viewport_extent = 4.0; // TODO(pat.m): should come from zoom
		let scale_factor = widget_extent / viewport_extent;

		(pos * scale_factor).to_egui_vec2()
	}

	fn paint_background(&self) {
		let rect = self.response.rect;
		let center = self.response.rect.center();

		self.painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
		self.painter.hline(rect.x_range(), center.y, (1.0, egui::Color32::DARK_GRAY));
		self.painter.vline(center.x, rect.y_range(), (1.0, egui::Color32::DARK_GRAY));
	}

	fn handle_hover(&mut self, hover_pos_world: Vec2) {
		let mut min_distance = 0.3;

		for &ViewportItem {shape, item, ..} in self.items.iter() {
			let distance = shape.distance_to(hover_pos_world);
			if distance < min_distance {
				self.state.hovered = Some(item);
				min_distance = distance;
			}
		}
	}

	fn handle_item_interaction(&mut self) {
		if self.response.drag_started_by(egui::PointerButton::Primary) {
			self.state.operation = self.state.hovered.map(Operation::Drag);
			self.state.selection = self.state.hovered;
		}

		if self.response.clicked() {
			self.state.selection = self.state.hovered;
		}

		if self.response.drag_released_by(egui::PointerButton::Primary) {
			self.state.operation = None;
		}
	}

	fn draw_items(&self) {
		for &ViewportItem{item, shape, color} in self.items.iter() {
			let item_hovered = self.state.hovered == Some(item);
			let color = color.to_egui_rgba();

			match shape {
				ViewportItemShape::Vertex(vertex) => {
					if let Item::Room(room_index) = item {
						let vertex_px = self.world_to_widget_pos(vertex);

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
						let vertex_px = self.world_to_widget_pos(vertex);
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

					let start = self.world_to_widget_pos(start);
					let end = self.world_to_widget_pos(end);

					self.painter.line_segment([start, end], (stroke_thickness, color));
				}
			}
		}
	}

	fn ui(mut self, _ui: &mut egui::Ui) -> egui::Response {
		self.paint_background();

		// Figure out what is hovered (if no operations are happening)
		if self.state.operation.is_none() {
			self.state.hovered = None;

			if let Some(hover_pos) = self.response.hover_pos() {
				self.handle_hover(self.widget_to_world_pos(hover_pos));
			}
		}

		self.handle_item_interaction();

		self.draw_items();

		// Perform operation
		match self.state.operation {
			Some(Operation::Drag(item)) => {
				let delta = self.widget_to_world_delta(self.response.drag_delta());

				if let Some(room) = self.world.rooms.get_mut(item.room_index()) {
					match item {
						Item::Vertex(GlobalVertexId {vertex_index, ..}) => {
							room.wall_vertices[vertex_index] += delta;
						}

						Item::Wall(GlobalWallId {wall_index, ..}) => {
							let wall_count = room.wall_vertices.len();
							room.wall_vertices[wall_index] += delta;
							room.wall_vertices[(wall_index+1) % wall_count] += delta;
						}

						Item::Room(_) => {
							for vertex in room.wall_vertices.iter_mut() {
								*vertex += delta;
							}
						}
					}

					self.response.mark_changed();
				}
			}

			_ => {}
		}

		self.response
	}
}

