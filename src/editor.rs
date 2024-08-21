use crate::prelude::*;
use world::{World, WorldChangedEvent, GlobalVertexId, GlobalWallId};

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
	Drag(Item),
}

impl Operation {
	fn relevant_item(&self) -> Option<Item> {
		match *self {
			Self::Drag(item) => Some(item),
		}
	}
}


#[derive(Copy, Clone, Default, Debug)]
struct State {
	hovered: Option<Item>,
	selection: Option<Item>,
	focused_room_index: usize,

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

			changed |= draw_focused_room_viewport(ui, &mut context).changed();
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
	let selected_room_index = state.selection.as_ref().map_or(state.focused_room_index, Item::room_index);

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



fn draw_focused_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let focused_room_index = context.state.selection.as_ref().map_or(context.state.focused_room_index, Item::room_index);

	let mut viewport = Viewport::new(ui, context);
	viewport.add_room_at(focused_room_index, Vec2::zero());
	viewport.ui(ui)
}

