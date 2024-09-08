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
	let mut transaction = state.undo_stack.transaction(model, message_bus);

	for cmd in message_bus.poll_consume(&state.editor_world_edit_cmd_sub) {
		if let Err(err) = handle_world_edit_cmd(&mut state.inner, &mut transaction, cmd) {
			log::error!("Editor command failed: {err}");
		}

		// Enable after the first command, so that all commands within a frame that can be merged are merged
		transaction.undo_stack.set_merging_enabled(true);
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

fn handle_world_edit_cmd(state: &mut InnerState, transaction: &mut Transaction<'_>, cmd: EditorWorldEditCmd) -> anyhow::Result<()> {
	if !matches!(cmd, EditorWorldEditCmd::TranslateItem(..)) {
		log::info!("{cmd:?}");
	}

	let model = &mut transaction.model;
	let undo_stack = &mut transaction.undo_stack;

	match cmd {
		EditorWorldEditCmd::TranslateItem(item, delta) => {
			// TODO(pat.m): this doesn't make sense for player spawn
			let room_index = item.room_index(&model.world);
			let Some(room) = model.world.rooms.get_mut(room_index) else {
				anyhow::bail!("Trying to edit non-existent room #{room_index}");
			};

			match item {
				Item::Vertex(VertexId {room_index, vertex_index}) => {
					transaction.update_room(room_index, |room| {
						room.wall_vertices[vertex_index] += delta;
					})?;
				}

				Item::Wall(WallId {wall_index, ..}) => {
					transaction.update_room(room_index, |room| {
						let wall_count = room.wall_vertices.len();
						room.wall_vertices[wall_index] += delta;
						room.wall_vertices[(wall_index+1) % wall_count] += delta;
					})?;
				}

				Item::Room(_) => {
					transaction.update_room(room_index, |room| {
						for vertex in room.wall_vertices.iter_mut() {
							*vertex += delta;
						}
					})?;
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
			transaction.update_room(room_index, |room| {
				room.ceiling_color = color;
			})?;
		}

		EditorWorldEditCmd::SetCeilingHeight(room_index, height) => {
			transaction.update_room(room_index, |room| {
				room.height = height;
			})?;
		}

		EditorWorldEditCmd::SetFloorColor(room_index, color) => {
			transaction.update_room(room_index, |room| {
				room.floor_color = color;
			})?;
		}

		EditorWorldEditCmd::SetWallColor(wall_id, color) => {
			transaction.update_wall(wall_id, |wall| {
				wall.color = color;
			})?;
		}

		EditorWorldEditCmd::SetHorizontalWallOffset(wall_id, offset) => {
			transaction.update_wall(wall_id, |wall| {
				wall.horizontal_offset = offset;
			})?;
		}

		EditorWorldEditCmd::SetVerticalWallOffset(wall_id, offset) => {
			transaction.update_wall(wall_id, |wall| {
				wall.vertical_offset = offset;
			})?;
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

			transaction.emit_world_changed();
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
			if let Some(selected_item) = &mut state.selection {
				// TODO(pat.m): this doesn't really make sense for player spawn
				let selected_room_index = selected_item.room_index(&model.world);

				if selected_room_index > room_index {
					selected_item.set_room_index(selected_room_index.saturating_sub(1));
				} else if selected_room_index == room_index {
					state.selection = None;
				}
			}

			// Update focused room
			if state.focused_room_index >= room_index {
				state.focused_room_index = state.focused_room_index.saturating_sub(1);
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

			transaction.emit_world_changed();
		}

		EditorWorldEditCmd::DisconnectRoom(room_index) => {
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a.room_index != room_index && wall_b.room_index != room_index
			});
			
			transaction.emit_world_changed();
		}

		EditorWorldEditCmd::ConnectWall(source_wall_id, target_wall_id) => {
			// Remove any connections to either the source or target walls
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a != source_wall_id && wall_b != source_wall_id
				&& wall_a != target_wall_id && wall_b != target_wall_id
			});

			// Connect
			model.world.connections.push((source_wall_id, target_wall_id));
			
			transaction.emit_world_changed();
		}

		EditorWorldEditCmd::DisconnectWall(wall_id) => {
			model.world.connections.retain(|&(wall_a, wall_b)| {
				wall_a != wall_id && wall_b != wall_id
			});
			
			transaction.emit_world_changed();
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
			
			transaction.emit_world_changed();
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
			
			transaction.emit_world_changed();
		}


		EditorWorldEditCmd::AddObject(object) => {
			model.world.objects.push(object);
			transaction.emit_world_changed();
		}

		EditorWorldEditCmd::RemoveObject(_object_index) => {
			todo!()
		}
	}

	Ok(())
}