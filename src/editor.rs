use crate::prelude::*;

use model::{Model, VertexId, WallId};

mod viewport;
use viewport::{Viewport, ViewportItemFlags};

mod commands;
pub use commands::*;

mod undo;
use undo::*;

mod world_editor;
pub use world_editor::draw_world_editor;

use std::borrow::Cow;
use egui::widgets::Slider;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Item {
	Vertex(VertexId),
	Wall(WallId),
	Room(usize),

	PlayerSpawn,
	Object(usize),
}

impl Item {
	fn room_index(&self, world: &model::World) -> usize {
		match *self {
			Item::Room(room_index) => room_index,
			Item::Vertex(VertexId{room_index, ..}) | Item::Wall(WallId{room_index, ..}) => room_index,
			Item::PlayerSpawn => world.player_spawn.room_index,
			Item::Object(object_index) => world.objects.get(object_index)
				.map_or(0, |obj| obj.placement.room_index),
		}
	}

	fn set_room_index(&mut self, new_room_index: usize) {
		match self {
			Item::Room(room_index) | Item::Vertex(VertexId{room_index, ..}) | Item::Wall(WallId{room_index, ..}) => {
				*room_index = new_room_index;
			}

			_ => {}
		}
	}
}


#[derive(Debug)]
pub struct State {
	inner: InnerState,
	undo_stack: UndoStack,

	editor_world_edit_cmd_sub: Subscription<EditorWorldEditCmd>,
	undo_cmd_sub: Subscription<UndoCmd>,
	// editor_modal_cmd_sub: Subscription<EditorModalCmd>,
}

#[derive(Debug)]
struct InnerState {
	hovered: Option<Item>,
	selection: Option<Item>,

	focused_room_index: usize,
	track_player: bool,
}

impl State {
	pub fn new(message_bus: &MessageBus) -> Self {
		State {
			inner: InnerState {
				hovered: None,
				selection: None,

				focused_room_index: 0,
				track_player: false,
			},

			undo_stack: UndoStack::new(message_bus.clone()),
			editor_world_edit_cmd_sub: message_bus.subscribe(),
			undo_cmd_sub: message_bus.subscribe(),
			// editor_modal_cmd_sub: message_bus.subscribe(),
		}
	}
}

struct Context<'w> {
	state: &'w mut InnerState,
	model: &'w model::Model,
	message_bus: &'w MessageBus,
}

fn validate_model(state: &mut State, model: &mut Model) {
	let num_rooms = model.world.rooms.len();

	if state.inner.focused_room_index >= num_rooms {
		state.inner.focused_room_index = 0;
	}

	if model.player.placement.room_index >= num_rooms {
		model.player.placement.room_index = 0;
	}

	// Yuck
	if model.world.player_spawn.room_index >= num_rooms {
		model.world.player_spawn.room_index = 0;
	}

	validate_item(model, &mut state.inner.hovered);
	validate_item(model, &mut state.inner.selection);
}


fn validate_item(model: &Model, maybe_item: &mut Option<Item>) {
	let num_rooms = model.world.rooms.len();
	let num_objects = model.world.objects.len();

	match maybe_item {
		Some(item) if item.room_index(&model.world) >= num_rooms => {
			*maybe_item = None;
		}

		&mut Some(Item::Object(object_index)) if object_index >= num_objects => {
			*maybe_item = None;
		}

		&mut Some(Item::Wall(WallId{room_index, wall_index: index}) | Item::Vertex(VertexId{room_index, vertex_index: index})) => {
			// Safe because room_index is already checked
			if index >= model.world.rooms[room_index].walls.len() {
				*maybe_item = None;
			}
		}

		_ => {}
	}
}