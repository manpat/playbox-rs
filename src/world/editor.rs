use crate::prelude::*;
use world::{World, WorldView, GlobalVertexId, GlobalWallId};

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

	selected_room: usize,
	operation: Option<Operation>,
}

struct Context<'w> {
	state: State,
	world: &'w mut World,
}

pub fn draw_world_editor(ctx: &egui::Context, world: &mut World, world_view: &mut WorldView) {
	let mut changed = false;

	let mut context = Context {
		state: ctx.data(|map| map.get_temp(egui::Id::null()).unwrap_or_default()),
		world,
	};

	egui::Window::new("World")
		.show(ctx, |ui| {
			changed |= ui.color_edit_button_rgb(context.world.fog_color.as_mut()).changed();
		});

	egui::Window::new("Viewport")
		.show(ctx, |ui| {
			draw_room_selector(ui, &mut context);
			changed |= draw_room_viewport(ui, &mut context);
		});

	egui::Window::new("Inspector")
		.show(ctx, |ui| {
			changed |= draw_item_inspector(ui, &mut context);
		});

	ctx.data_mut(move |map| map.insert_temp(egui::Id::null(), context.state));

	if changed {
		world_view.needs_rebuild = true;
	}
}


fn draw_room_selector(ui: &mut egui::Ui, Context{world, state}: &mut Context) {
	ui.horizontal(|ui| {
		for (idx, room) in world.rooms.iter().enumerate() {
			let selected = idx == state.selected_room;
			if ui.selectable_label(selected, format!("{idx}")).clicked() {
				state.selected_room = idx;
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

fn draw_room_viewport(ui: &mut egui::Ui, Context{world, state}: &mut Context) -> bool {
	let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), ui.available_width()), egui::Sense::click_and_drag());
	let rect = response.rect;
	let center = response.rect.center();

	painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
	painter.hline(rect.x_range(), center.y, (1.0, egui::Color32::DARK_GRAY));
	painter.vline(center.x, rect.y_range(), (1.0, egui::Color32::DARK_GRAY));
	let local_extent = 4.0;

	let room_index = state.selected_room;
	let Some(room) = world.rooms.get_mut(room_index) else {
		return false
	};

	let widget_extent = rect.size().x / 2.0;
	let scale_factor = widget_extent / local_extent;
	let num_walls = room.walls.len();
	let room_center = room.wall_vertices.iter().sum::<Vec2>() / num_walls as f32;

	// Figure out what is hovered (if no operations are happening)
	if state.operation.is_none() {
		state.hovered = None;
		if let Some(hover_pos) = response.hover_pos() {
			let local_pos = (hover_pos - center) / scale_factor;
			let local_pos = Vec2::from_compatible(local_pos);

			let mut min_distance = 0.3;

			// Pick vertices
			for (vertex_index, vertex) in room.wall_vertices.iter().enumerate() {
				let distance = (*vertex - local_pos).length();
				if distance < min_distance {
					state.hovered = Some(Item::Vertex(GlobalVertexId {room_index, vertex_index}));
					min_distance = distance;
				}
			}

			// Pick walls
			for wall_index in 0..num_walls {
				let (start, end) = room.wall_vertices(wall_index);

				let wall_diff = end - start;
				let wall_length = wall_diff.length();
				let wall_direction = wall_diff / wall_length;

				let delta = local_pos - start;

				let distance_along = wall_direction.dot(delta) / wall_length;
				let distance_across = wall_direction.wedge(delta).abs();

				let buffer = 0.2;

				if distance_along < 0.0 || distance_along > 1.0 || distance_across >= min_distance {
					continue;
				}

				let point_a = start + wall_direction * wall_length * buffer;
				let point_b = end - wall_direction * wall_length * buffer;

				let dist_a = (point_a - local_pos).length();
				let dist_b = (point_b - local_pos).length();

				let mut distance = f32::INFINITY;

				if distance_along >= buffer && distance_along <= 1.0 - buffer {
					distance = distance.min(distance_across);
				}

				distance = distance.min(dist_a).min(dist_b);

				if distance < min_distance {
					state.hovered = Some(Item::Wall(GlobalWallId {room_index, wall_index}));
					min_distance = distance;
				}
			}

			// Pick room
			let distance = (room_center - local_pos).length();
			if distance < min_distance {
				state.hovered = Some(Item::Room(room_index));
			}
		}
	}


	// Handle state transitions
	if response.drag_started_by(egui::PointerButton::Primary) {
		state.operation = state.hovered.map(Operation::Drag);
		state.selection = state.hovered;
	}

	if response.clicked() {
		state.selection = state.hovered;
	}

	if response.drag_released_by(egui::PointerButton::Primary) {
		state.operation = None;
	}


	// Draw
	let room_center_px = center + (room_center * scale_factor).to_egui_vec2();
	painter.text(
		room_center_px,
		egui::Align2::CENTER_CENTER,
		format!("#{room_index}"),
		egui::FontId::proportional(12.0),
		if state.hovered == Some(Item::Room(room_index)) {
			egui::Color32::WHITE
		} else {
			egui::Color32::GRAY
		}
	);

	for wall_index in 0..num_walls {
		let (start, end) = room.wall_vertices(wall_index);
		let start = start * scale_factor;
		let end = end * scale_factor;

		let start = center + start.to_egui_vec2();
		let end = center + end.to_egui_vec2();

		let id = GlobalWallId {room_index, wall_index};

		let wall_hovered = state.hovered == Some(Item::Wall(id));
		let stroke_thickness = match wall_hovered {
			false => 1.0,
			true => 4.0,
		};

		let color = room.walls[wall_index].color.to_egui_rgba();

		painter.line_segment([start, end], (stroke_thickness, color));
	}

	if response.hovered() {
		for (vertex_index, vertex) in room.wall_vertices.iter_mut().enumerate() {
			let vertex_px = *vertex * scale_factor;

			let rect = egui::Rect::from_center_size(center + vertex_px.to_egui_vec2(), egui::vec2(12.0, 12.0));

			let id = GlobalVertexId {room_index, vertex_index};
			let vertex_hovered = state.hovered == Some(Item::Vertex(id));

			if vertex_hovered {
				painter.rect_filled(rect, 0.0, egui::Color32::GRAY);
			} else {
				painter.rect_stroke(rect, 0.0, (1.0, egui::Color32::GRAY));
			}
		}
	}

	// Perform operation
	match state.operation {
		Some(Operation::Drag(Item::Vertex(GlobalVertexId {room_index, vertex_index}))) => {
			if let Some(room) = world.rooms.get_mut(room_index) {
				let delta = Vec2::from_compatible(response.drag_delta()) / scale_factor;

				room.wall_vertices[vertex_index] += delta;

				true
			} else {
				false
			}
		}

		Some(Operation::Drag(Item::Wall(GlobalWallId {room_index, wall_index}))) => {
			if let Some(room) = world.rooms.get_mut(room_index) {
				let delta = Vec2::from_compatible(response.drag_delta()) / scale_factor;

				let wall_count = room.wall_vertices.len();

				room.wall_vertices[wall_index] += delta;
				room.wall_vertices[(wall_index+1) % wall_count] += delta;

				true
			} else {
				false
			}
		}

		_ => false
	}
}
