// use crate::prelude::*;
use model::{WorldChangedEvent, Room, Object, VertexId, WallId};
use super::*;


#[derive(Debug)]
pub enum EditorWorldEditCmd {
	TranslateItem(Item, Vec2),

	SetCeilingColor(usize, Color),
	SetCeilingHeight(usize, f32),
	SetFloorColor(usize, Color),

	SetWallColor(WallId, Color),
	SetHorizontalWallOffset(WallId, f32),
	SetVerticalWallOffset(WallId, f32),

	SetFogParams(Color),


	AddRoom {
		room: Room,
		connection: Option<(usize, WallId)>,
	},

	RemoveRoom(usize),
	DisconnectRoom(usize),

	ConnectWall(WallId, WallId),
	DisconnectWall(WallId),

	SplitWall(WallId, Vec2),
	DeleteVertex(VertexId),


	// TODO(pat.m): could be an object
	SetPlayerSpawn,

	AddObject(Object),
	RemoveObject(usize),
}


#[derive(Copy, Clone, Debug)]
pub enum UndoCmd {
	Undo,
	Redo,
	SetIndex(usize),
}


pub fn handle_editor_cmds(state: &mut State, model: &mut model::Model, message_bus: &MessageBus) {
	// Handle undo/redo
	for cmd in message_bus.poll(&state.undo_cmd_sub) {
		if let Err(err) = handle_undo_cmd(&mut state.undo_stack, model, cmd) {
			log::error!("{cmd:?} failed: {err}");
		}
	}

	// Handle editor commands
	let messages_available = message_bus.messages_available(&state.editor_world_edit_cmd_sub);

	for cmd in message_bus.poll_consume(&state.editor_world_edit_cmd_sub) {
		if let Err(err) = handle_world_edit_cmd(state, model, cmd) {
			log::error!("Editor command failed: {err}");
		}

		// Enable after the first command, so that all commands within a frame that can be merged are merged
		state.undo_stack.set_merging_enabled(true);
	}

	// state.undo_stack.set_merging_enabled(false);

	if messages_available {
		message_bus.emit(WorldChangedEvent);
	}
}


fn handle_undo_cmd(undo_stack: &mut UndoStack, model: &mut model::Model, cmd: UndoCmd) -> anyhow::Result<()> {
	match cmd {
		UndoCmd::Undo => undo_stack.undo(model),
		UndoCmd::Redo => undo_stack.redo(model),
		UndoCmd::SetIndex(index) => undo_stack.set_index(model, index),
	}

	Ok(())
}

fn handle_world_edit_cmd(state: &mut State, model: &mut model::Model, cmd: EditorWorldEditCmd) -> anyhow::Result<()> {
	if !matches!(cmd, EditorWorldEditCmd::TranslateItem(..)) {
		log::info!("{cmd:?}");
	}

	match cmd {
		EditorWorldEditCmd::TranslateItem(item, delta) => {
			// TODO(pat.m): this doesn't make sense for player spawn
			let room_index = item.room_index(&model.world);
			let Some(room) = model.world.rooms.get_mut(room_index) else {
				anyhow::bail!("Trying to edit non-existent room #{room_index}");
			};

			match item {
				Item::Vertex(VertexId {vertex_index, ..}) => {
					let before = room.clone();
					room.wall_vertices[vertex_index] += delta;

					state.undo_stack.push(UndoEntry::UpdateRoom {room_index, before, after: room.clone()});
				}

				Item::Wall(WallId {wall_index, ..}) => {
					let before = room.clone();
					let wall_count = room.wall_vertices.len();
					room.wall_vertices[wall_index] += delta;
					room.wall_vertices[(wall_index+1) % wall_count] += delta;

					state.undo_stack.push(UndoEntry::UpdateRoom {room_index, before, after: room.clone()});
				}

				Item::Room(_) => {
					let before = room.clone();
					for vertex in room.wall_vertices.iter_mut() {
						*vertex += delta;
					}

					state.undo_stack.push(UndoEntry::UpdateRoom {room_index, before, after: room.clone()});
				}

				Item::Object(index) => {
					let object = model.world.objects.get_mut(index)
						.with_context(|| format!("Trying to edit non-existent object #{index}"))?;

					object.placement.position += delta;

					// TODO(pat.m): need to be able to move between rooms!
				}

				Item::PlayerSpawn => {
					todo!()
				}
			}
		}

		EditorWorldEditCmd::SetCeilingColor(room_index, color) => {
			if let Some(room) = model.world.rooms.get_mut(room_index) {
				let before = room.clone();
				room.ceiling_color = color;
				state.undo_stack.push(UndoEntry::UpdateRoom {room_index, before, after: room.clone()});
			} else {
				anyhow::bail!("Trying to edit non-existent room #{room_index}");
			}
		}

		EditorWorldEditCmd::SetCeilingHeight(room_index, height) => {
			if let Some(room) = model.world.rooms.get_mut(room_index) {
				let before = room.clone();
				room.height = height;
				state.undo_stack.push(UndoEntry::UpdateRoom {room_index, before, after: room.clone()});
			} else {
				anyhow::bail!("Trying to edit non-existent room #{room_index}");
			}
		}

		EditorWorldEditCmd::SetFloorColor(room_index, color) => {
			if let Some(room) = model.world.rooms.get_mut(room_index) {
				let before = room.clone();
				room.floor_color = color;
				state.undo_stack.push(UndoEntry::UpdateRoom {room_index, before, after: room.clone()});
			} else {
				anyhow::bail!("Trying to edit non-existent room #{room_index}");
			}
		}

		EditorWorldEditCmd::SetWallColor(wall_id, color) => {
			if let Some(wall) = model.world.rooms.get_mut(wall_id.room_index)
				.and_then(|room| room.walls.get_mut(wall_id.wall_index))
			{
				let before = wall.clone();
				wall.color = color;
				state.undo_stack.push(UndoEntry::UpdateWall {wall_id, before, after: wall.clone()});
			} else {
				anyhow::bail!("Trying to edit non-existent wall {wall_id:?}");
			}
		}

		EditorWorldEditCmd::SetHorizontalWallOffset(wall_id, offset) => {
			if let Some(wall) = model.world.rooms.get_mut(wall_id.room_index)
				.and_then(|room| room.walls.get_mut(wall_id.wall_index))
			{
				let before = wall.clone();
				wall.horizontal_offset = offset;
				state.undo_stack.push(UndoEntry::UpdateWall {wall_id, before, after: wall.clone()});
			} else {
				anyhow::bail!("Trying to edit non-existent wall {wall_id:?}");
			}
		}

		EditorWorldEditCmd::SetVerticalWallOffset(wall_id, offset) => {
			if let Some(wall) = model.world.rooms.get_mut(wall_id.room_index)
				.and_then(|room| room.walls.get_mut(wall_id.wall_index))
			{
				let before = wall.clone();
				wall.vertical_offset = offset;
				state.undo_stack.push(UndoEntry::UpdateWall {wall_id, before, after: wall.clone()});
			} else {
				anyhow::bail!("Trying to edit non-existent wall {wall_id:?}");
			}
		}

		EditorWorldEditCmd::SetFogParams(color) => {
			model.world.fog_color = color;
		}

		EditorWorldEditCmd::SetPlayerSpawn => {
			model.world.player_spawn = model.player.placement;
		}

		EditorWorldEditCmd::AddRoom { room, connection } => {
			let new_room_index = model.world.rooms.len();

			// Unnecessary clone :(
			// TODO(pat.m): consuming subscriptions
			model.world.rooms.push(room);

			if let Some((source_wall_index, target_wall_id)) = connection {
				// Disconnect target wall
				model.world.connections.retain(|&(wall_a, wall_b)| {
					wall_a != target_wall_id && wall_b != target_wall_id
				});

				let source_wall_id = WallId {
					room_index: new_room_index,
					wall_index: source_wall_index,
				};

				// Create new connection
				model.world.connections.push((source_wall_id, target_wall_id));
			}
		}

		EditorWorldEditCmd::RemoveRoom(room_index) => {
			// TODO(pat.m): maybe find a way to do this that _doesn't_ involve touching every Location in the model

			if model.world.rooms.len() == 1 {
				anyhow::bail!("Can't delete last room in world")
			}

			if model.player.placement.room_index == room_index {
				anyhow::bail!("Can't delete room containing player");
			}


			// Fix player position
			if model.player.placement.room_index > room_index {
				model.player.placement.room_index = model.player.placement.room_index.saturating_sub(1);
			}

			// Fix player spawn
			if model.world.player_spawn.room_index > room_index {
				model.world.player_spawn.room_index = model.world.player_spawn.room_index.saturating_sub(1);
			}

			// Clear or adjust selection
			if let Some(selected_item) = &mut state.inner.selection {
				// TODO(pat.m): this doesn't really make sense for player spawn
				let selected_room_index = selected_item.room_index(&model.world);

				if selected_room_index > room_index {
					selected_item.set_room_index(selected_room_index.saturating_sub(1));
				} else if selected_room_index == room_index {
					state.inner.selection = None;
				}
			}

			// Update focused room
			if state.inner.focused_room_index >= room_index {
				state.inner.focused_room_index = state.inner.focused_room_index.saturating_sub(1);
			}

			// Remove connections to deleted room
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a.room_index != room_index && wall_b.room_index != room_index
			});

			// Update all connections with corrected room indices
			for (wall_a, wall_b) in model.world.connections.iter_mut() {
				if wall_a.room_index > room_index {
					wall_a.room_index -= 1;
				}

				if wall_b.room_index > room_index {
					wall_b.room_index -= 1;
				}
			}

			// Actually remove room
			model.world.rooms.remove(room_index);
		}

		EditorWorldEditCmd::DisconnectRoom(room_index) => {
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a.room_index != room_index && wall_b.room_index != room_index
			});
		}

		EditorWorldEditCmd::ConnectWall(source_wall_id, target_wall_id) => {
			// Remove any connections to either the source or target walls
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a != source_wall_id && wall_b != source_wall_id
				&& wall_a != target_wall_id && wall_b != target_wall_id
			});

			// Connect
			model.world.connections.push((source_wall_id, target_wall_id));
		}

		EditorWorldEditCmd::DisconnectWall(wall_id) => {
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a != wall_id && wall_b != wall_id
			});
		}

		EditorWorldEditCmd::SplitWall(wall_id, new_position) => {
			let room = model.world.rooms.get_mut(wall_id.room_index)
				.context("Invalid room index")?;

			let wall = room.walls.get(wall_id.wall_index)
				.context("Invalid wall index")?
				.clone();

			let new_wall_index = wall_id.wall_index + 1;

			// Insert the new wall after the target wall
			room.walls.insert(new_wall_index, wall);
			room.wall_vertices.insert(new_wall_index, new_position);

			// Update all connections with corrected wall ids
			for (wall_a, wall_b) in model.world.connections.iter_mut() {
				if wall_a.room_index == wall_id.room_index && wall_a.wall_index >= new_wall_index {
					wall_a.wall_index += 1;
				}

				if wall_b.room_index == wall_id.room_index && wall_b.wall_index >= new_wall_index {
					wall_b.wall_index += 1;
				}
			}
		}

		EditorWorldEditCmd::DeleteVertex(vertex_id) => {
			let room = model.world.rooms.get_mut(vertex_id.room_index)
				.context("Invalid room index")?;

			if vertex_id.vertex_index >= room.walls.len() {
				anyhow::bail!("Trying to delete invalid vertex");
			}

			room.walls.remove(vertex_id.vertex_index);
			room.wall_vertices.remove(vertex_id.vertex_index);

			let wall_id = vertex_id.to_wall_id();

			// Remove connections to deleted wall
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a != wall_id && wall_b != wall_id
			});

			// Update all connections with corrected wall ids
			for (wall_a, wall_b) in model.world.connections.iter_mut() {
				if wall_a.room_index == vertex_id.room_index && wall_a.wall_index > vertex_id.vertex_index {
					wall_a.wall_index -= 1;
				}

				if wall_b.room_index == vertex_id.room_index && wall_b.wall_index > vertex_id.vertex_index {
					wall_b.wall_index -= 1;
				}
			}
		}


		EditorWorldEditCmd::AddObject(object) => {
			model.world.objects.push(object);
		}

		EditorWorldEditCmd::RemoveObject(_object_index) => {
			todo!()
		}
	}

	Ok(())
}