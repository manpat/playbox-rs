use crate::prelude::*;

use model::{Model, VertexId, WallId, RoomId};

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
	Room(RoomId),

	PlayerSpawn,
	Object(usize),
}

impl Item {
	fn room_id(&self, world: &model::World) -> RoomId {
		let geometry = &world.geometry;

		match *self {
			Item::Room(room_id) => room_id,
			Item::Vertex(vertex_id) => {
				let wall_id = geometry.vertices[vertex_id].outgoing_wall;
				geometry.walls[wall_id].room
			}
			Item::Wall(wall_id) => geometry.walls[wall_id].room,
			Item::PlayerSpawn => world.player_spawn.room_id,
			Item::Object(object_index) => world.objects.get(object_index)
				.map(|obj| obj.placement.room_id)
				.unwrap(),
		}
	}
}


#[derive(Debug)]
pub struct State {
	inner: InnerState,
	pub undo_stack: UndoStack,

	editor_world_edit_cmd_sub: Subscription<EditorWorldEditCmd>,
	undo_cmd_sub: Subscription<UndoCmd>,
	// editor_modal_cmd_sub: Subscription<EditorModalCmd>,
}

#[derive(Debug)]
struct InnerState {
	hovered: Option<Item>,
	selection: Option<Item>,

	focused_room_id: Option<RoomId>,
	track_player: bool,
}

impl State {
	pub fn new(message_bus: &MessageBus) -> Self {
		State {
			inner: InnerState {
				hovered: None,
				selection: None,

				focused_room_id: None,
				track_player: false,
			},

			undo_stack: UndoStack::new(message_bus.clone()),
			editor_world_edit_cmd_sub: message_bus.subscribe(),
			undo_cmd_sub: message_bus.subscribe(),
			// editor_modal_cmd_sub: message_bus.subscribe(),
		}
	}

	pub fn reset(&mut self) {
		self.undo_stack.clear();
		self.inner.focused_room_id = None;
		self.inner.hovered = None;
		self.inner.selection = None;
	}
}

struct Context<'w> {
	state: &'w mut InnerState,
	model: &'w model::Model,
	message_bus: &'w MessageBus,
}

fn validate_model(state: &mut State, model: &mut Model) {
	let geometry = &model.world.geometry;
	let first_room = geometry.first_room();

	if !state.inner.focused_room_id.map_or(false, |room_id| room_id.is_valid(geometry)) {
		state.inner.focused_room_id = Some(first_room);
	}

	// TODO(pat.m): needs to happen somewhere else
	// if !model.player.placement.room_id.is_valid(geometry) {
	// 	model.player.placement.room_id = first_room;
	// }

	// Yuck
	if !model.world.player_spawn.room_id.is_valid(geometry) {
		model.world.player_spawn.room_id = first_room;
	}

	validate_item(model, &mut state.inner.hovered);
	validate_item(model, &mut state.inner.selection);
}


fn validate_item(model: &Model, maybe_item: &mut Option<Item>) {
	let geometry = &model.world.geometry;
	let num_objects = model.world.objects.len();

	match maybe_item {
		Some(item) if !geometry.rooms.contains_key(item.room_id(&model.world)) => {
			*maybe_item = None;
		}

		&mut Some(Item::Object(object_index)) if object_index >= num_objects => {
			*maybe_item = None;
		}

		&mut Some(Item::Wall(wall_id)) if !geometry.walls.contains_key(wall_id) => {
			*maybe_item = None;
		}

		&mut Some(Item::Vertex(vertex_id)) if !geometry.vertices.contains_key(vertex_id) => {
			*maybe_item = None;
		}

		_ => {}
	}
}

