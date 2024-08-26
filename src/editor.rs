use crate::prelude::*;

use model::{WorldChangedEvent, GlobalVertexId, GlobalWallId};

mod viewport;
use viewport::Viewport;


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
	Drag {
		item: Item,
		room_to_world: Mat2x3,
	},
}

impl Operation {
	fn relevant_item(&self) -> Option<Item> {
		match *self {
			Self::Drag{item, ..} => Some(item),
		}
	}
}


#[derive(Debug)]
pub struct State {
	hovered: Option<Item>,
	hovered_transform: Option<Mat2x3>,

	selection: Option<Item>,
	focused_room_index: usize,

	current_operation: Option<Operation>,

	editor_cmd_sub: Subscription<EditorCmd>,
}

impl State {
	pub fn new(message_bus: &MessageBus) -> Self {
		State {
			hovered: None,
			hovered_transform: None,

			selection: None,
			focused_room_index: 0,

			current_operation: None,

			editor_cmd_sub: message_bus.subscribe(),
		}
	}
}

struct Context<'w> {
	state: &'w mut State,
	model: &'w model::Model,
	message_bus: &'w MessageBus,
}

pub fn draw_world_editor(ctx: &egui::Context, state: &mut State, model: &model::Model, message_bus: &MessageBus) {
	let mut context = Context {
		state,
		model,
		message_bus,
	};

	egui::Window::new("World")
		.show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.label("Fog Color");

				let mut fog_color = model.world.fog_color;
				if ui.color_edit_button_rgb(fog_color.as_mut()).changed() {
					message_bus.emit(EditorCmd::SetFogParams(fog_color));
				}
			});
		});

	egui::Window::new("Viewport")
		.show(ctx, |ui| {
			draw_room_selector(ui, &mut context);
			draw_focused_room_viewport(ui, &mut context);
		});

	egui::Window::new("Inspector")
		.show(ctx, |ui| {
			draw_item_inspector(ui, &mut context);
		});
}


pub fn handle_editor_cmds(state: &State, model: &mut model::Model, message_bus: &MessageBus) {
	let mut changed = false;

	for cmd in message_bus.poll(&state.editor_cmd_sub).iter() {
		match cmd {
			&EditorCmd::TranslateItem(item, delta) => {
				if let Some(room) = model.world.rooms.get_mut(item.room_index()) {
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
				}
			}

			&EditorCmd::SetCeilingColor(room_index, color) => {
				if let Some(room) = model.world.rooms.get_mut(room_index) {
					room.ceiling_color = color;
				}
			}

			&EditorCmd::SetFloorColor(room_index, color) => {
				if let Some(room) = model.world.rooms.get_mut(room_index) {
					room.floor_color = color;
				}
			}

			&EditorCmd::SetWallColor(wall_id, color) => {
				if let Some(wall) = model.world.rooms.get_mut(wall_id.room_index)
					.and_then(|room| room.walls.get_mut(wall_id.wall_index))
				{
					wall.color = color;
				}
			}

			&EditorCmd::SetFogParams(color) => {
				model.world.fog_color = color;
			}
		}

		changed = true;
	}

	if changed {
		message_bus.emit(WorldChangedEvent);
	}
}


fn draw_room_selector(ui: &mut egui::Ui, Context{model, state, ..}: &mut Context) {
	let selected_room_index = state.selection.as_ref().map_or(state.focused_room_index, Item::room_index);

	ui.horizontal(|ui| {
		for (room_index, _room) in model.world.rooms.iter().enumerate() {
			let selected = room_index == selected_room_index;
			if ui.selectable_label(selected, format!("{room_index}")).clicked() {
				state.selection = Some(Item::Room(room_index));
			}
		}
	});
}

fn draw_item_inspector(ui: &mut egui::Ui, ctx: &mut Context) {
	match ctx.state.selection {
		None => {
			ui.label("<select an item>");
		}

		Some(Item::Vertex(vertex_id)) => {
			draw_room_inspector(ui, ctx, vertex_id.room_index);
		}

		Some(Item::Wall(wall_id)) => {
			draw_room_inspector(ui, ctx, wall_id.room_index);
			ui.separator();
			draw_wall_inspector(ui, ctx, wall_id);
		}

		Some(Item::Room(room_index)) => {
			draw_room_inspector(ui, ctx, room_index);
		}
	}

}

fn draw_room_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, room_index: usize) {
	let Some(room) = model.world.rooms.get(room_index) else {
		return
	};

	ui.label(format!("Room #{room_index}"));

	ui.horizontal(|ui| {
		ui.label("Ceiling");

		let mut ceiling_color = room.ceiling_color;
		if ui.color_edit_button_rgb(ceiling_color.as_mut()).changed() {
			message_bus.emit(EditorCmd::SetCeilingColor(room_index, ceiling_color));
		}
	});

	ui.horizontal(|ui| {
		ui.label("Floor");

		let mut floor_color = room.floor_color;
		if ui.color_edit_button_rgb(floor_color.as_mut()).changed() {
			message_bus.emit(EditorCmd::SetFloorColor(room_index, floor_color));
		}
	});
}

fn draw_wall_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, wall_id: GlobalWallId) {
	let GlobalWallId{room_index, wall_index} = wall_id;

	let Some(wall) = model.world.rooms.get(room_index)
		.and_then(|room| room.walls.get(wall_index))
	else {
		return
	};

	ui.label(format!("Wall #{wall_index}"));

	ui.horizontal(|ui| {
		ui.label("Wall");
		
		let mut wall_color = wall.color;
		if ui.color_edit_button_rgb(wall_color.as_mut()).changed() {
			message_bus.emit(EditorCmd::SetWallColor(wall_id, wall_color));
		}
	});
}



fn draw_focused_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let focused_room_index = context.state.selection.as_ref().map_or(context.state.focused_room_index, Item::room_index);

	let mut neighbouring_rooms = Vec::new();

	for wall_index in 0..context.model.world.rooms[focused_room_index].walls.len() {
		let src_wall_id = GlobalWallId{room_index: focused_room_index, wall_index};
		let Some(tgt_wall_id) = context.model.world.wall_target(src_wall_id)
			else { continue };

		let (start, end) = context.model.world.wall_vertices(src_wall_id);
		let wall_normal = (end - start).normalize().perp();

		let transform = model::calculate_portal_transform(&context.model.world, src_wall_id, tgt_wall_id);
		let offset_transform = Mat2x3::translate(wall_normal * 0.3) * transform;

		neighbouring_rooms.push((tgt_wall_id.room_index, offset_transform));
	}

	let mut viewport = Viewport::new(ui, context);
	viewport.add_room(focused_room_index, Mat2x3::identity());
	viewport.add_room_connections(focused_room_index, Mat2x3::identity());

	for (room_index, transform) in neighbouring_rooms {
		viewport.add_room(room_index, transform);
	}

	viewport.build()
}




#[derive(Debug)]
pub enum EditorCmd {
	TranslateItem(Item, Vec2),

	SetCeilingColor(usize, Color),
	SetFloorColor(usize, Color),
	SetWallColor(GlobalWallId, Color),

	SetFogParams(Color),
}