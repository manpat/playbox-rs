use crate::prelude::*;
use world::{World, WorldView};

#[derive(Copy, Clone, Default, Debug)]
enum DragState {
	#[default]
	None,

	Vertex {
		index: usize,
	}
}

#[derive(Copy, Clone, Default, Debug)]
struct State {
	selection: usize,
	drag: DragState,
}

struct Context<'w> {
	state: State,
	world: &'w mut World,
}

pub fn draw_world_editor(ui: &mut egui::Ui, world: &mut World, world_view: &mut WorldView) {
	let data_id = ui.next_auto_id();

	let mut context = Context {
		state: ui.data(|map| map.get_temp(data_id).unwrap_or_default()),
		world,
	};

	let mut changed = false;

	draw_room_selector(ui, &mut context);
	changed |= draw_room_viewport(ui, &mut context);

	ui.data_mut(move |map| map.insert_temp(data_id, context.state));

	if changed {
		world_view.needs_rebuild = true;
	}
}


fn draw_room_selector(ui: &mut egui::Ui, Context{world, state}: &mut Context) {
	for (idx, room) in world.rooms.iter().enumerate() {
		let selected = idx == state.selection;
		if ui.selectable_label(selected, format!("{idx}: {:?}", room.floor_color)).clicked() {
			state.selection = idx;
		}
	}
}


fn draw_room_viewport(ui: &mut egui::Ui, Context{world, state}: &mut Context) -> bool {
	let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), ui.available_width()), egui::Sense::click_and_drag());
	let rect = response.rect;
	let center = response.rect.center();

	painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
	painter.hline(rect.x_range(), center.y, (1.0, egui::Color32::DARK_GRAY));
	painter.vline(center.x, rect.y_range(), (1.0, egui::Color32::DARK_GRAY));
	let local_extent = 4.0;

	let Some(room) = world.rooms.get_mut(state.selection) else {
		return false
	};

	let widget_extent = rect.size().x / 2.0;
	let scale_factor = widget_extent / local_extent;
	let center = center.to_vec2();

	let num_walls = room.walls.len();

	for wall_idx in 0..num_walls {
		let (start, end) = room.wall_vertices(wall_idx);
		let start = start * scale_factor;
		let end = end * scale_factor;

		let start = egui::Pos2::from(start.to_tuple()) + center;
		let end = egui::Pos2::from(end.to_tuple()) + center;

		let (r, g, b) = room.walls[wall_idx].color.to_srgb().into();
		let color = egui::Color32::from_rgb(r, g, b);

		painter.line_segment([start, end], (1.0, color));
	}

	if response.drag_released_by(egui::PointerButton::Primary) {
		state.drag = DragState::None;
	}

	if let Some(hover_pos) = response.hover_pos() {
		// let hover_pos_room = (hover_pos - center) / scale_factor;

		// TODO(pat.m): separate figuring out hovered item from drawing

		for (index, vertex) in room.wall_vertices.iter_mut().enumerate() {
			let vertex_px = *vertex * scale_factor;
			let vertex_px = egui::Pos2::from(vertex_px.to_tuple());

			let rect = egui::Rect::from_center_size(vertex_px + center, egui::vec2(12.0, 12.0));

			let vertex_hovered = rect.contains(hover_pos);
			if vertex_hovered && response.drag_started_by(egui::PointerButton::Primary) {
				state.drag = DragState::Vertex {index};
			}

			if vertex_hovered {
				painter.rect_filled(rect, 0.0, egui::Color32::GRAY);
			} else {
				painter.rect_stroke(rect, 0.0, (1.0, egui::Color32::GRAY));
			}
		}
	}

	match state.drag {
		DragState::Vertex{index, ..} => {
			let delta_px = response.drag_delta();
			let delta = Vec2::new(delta_px.x, delta_px.y) / scale_factor;

			room.wall_vertices[index] += delta;

			true
		}

		_ => false
	}
}