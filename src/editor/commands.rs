// use crate::prelude::*;
use model::{Room, Object, VertexId, WallId};
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

	let should_merge_previous_frames = state.undo_stack.time_since_last_command().as_secs_f32() < 0.4;

	state.undo_stack.set_merging_enabled(should_merge_previous_frames);

	// Handle editor commands
	let mut transaction = state.undo_stack.transaction(model, message_bus);

	for cmd in message_bus.poll_consume(&state.editor_world_edit_cmd_sub) {
		if let Err(err) = handle_world_edit_cmd(&mut state.inner, &mut transaction, cmd) {
			log::error!("Editor command failed: {err}");
		}

		// Enable after the first command, so that all commands within a frame that can be merged are merged
		transaction.undo_stack.set_merging_enabled(true);
	}

	// Make sure we haven't messed anything up
	super::validate_model(state, model);
}


fn handle_undo_cmd(undo_stack: &mut UndoStack, model: &mut model::Model, cmd: UndoCmd) -> anyhow::Result<()> {
	log::info!("{cmd:?}");

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

	match cmd {
		EditorWorldEditCmd::TranslateItem(item, delta) => {
			match item {
				Item::Vertex(VertexId {room_index, vertex_index}) => {
					transaction.update_room(room_index, |room| {
						room.wall_vertices[vertex_index] += delta;

						Ok(())
					})?;
				}

				Item::Wall(WallId {room_index, wall_index}) => {
					transaction.update_room(room_index, |room| {
						let wall_count = room.wall_vertices.len();
						room.wall_vertices[wall_index] += delta;
						room.wall_vertices[(wall_index+1) % wall_count] += delta;

						Ok(())
					})?;
				}

				Item::Room(room_index) => {
					transaction.update_room(room_index, |room| {
						for vertex in room.wall_vertices.iter_mut() {
							*vertex += delta;
						}

						Ok(())
					})?;
				}

				Item::Object(index) => {
					// TODO(pat.m): :(
					transaction.update_world(|world| {
						let object = world.objects.get_mut(index)
							.with_context(|| format!("Trying to edit non-existent object #{index}"))?;

						// TODO(pat.m): need to be able to move between rooms!
						object.placement.position += delta;

						Ok(())
					})?;
				}

				Item::PlayerSpawn => {
					todo!()
				}
			}
		}

		EditorWorldEditCmd::SetCeilingColor(room_index, color) => {
			transaction.update_room(room_index, |room| {
				room.ceiling_color = color;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SetCeilingHeight(room_index, height) => {
			transaction.update_room(room_index, |room| {
				room.height = height;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SetFloorColor(room_index, color) => {
			transaction.update_room(room_index, |room| {
				room.floor_color = color;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SetWallColor(wall_id, color) => {
			transaction.update_wall(wall_id, |wall| {
				wall.color = color;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SetHorizontalWallOffset(wall_id, offset) => {
			transaction.update_wall(wall_id, |wall| {
				wall.horizontal_offset = offset;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SetVerticalWallOffset(wall_id, offset) => {
			transaction.update_wall(wall_id, |wall| {
				wall.vertical_offset = offset;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SetFogParams(color) => {
			transaction.update_world(|world| {
				world.fog_color = color;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SetPlayerSpawn => {
			let new_player_spawn = model.player.placement;
			transaction.update_world(|world| {
				world.player_spawn = new_player_spawn;
				Ok(())
			})?;
		}

		EditorWorldEditCmd::AddRoom { room, connection } => {
			transaction.update_world(|world| {
				let new_room_index = world.rooms.len();

				world.rooms.push(room);

				if let Some((source_wall_index, target_wall_id)) = connection {
					// Disconnect target wall
					world.connections.retain(|&(wall_a, wall_b)| {
						wall_a != target_wall_id && wall_b != target_wall_id
					});

					let source_wall_id = WallId {
						room_index: new_room_index,
						wall_index: source_wall_index,
					};

					// Create new connection
					world.connections.push((source_wall_id, target_wall_id));
				}

				Ok(())
			})?;
		}

		EditorWorldEditCmd::RemoveRoom(room_index) => {
			if model.world.rooms.len() == 1 {
				anyhow::bail!("Can't delete last room in world")
			}

			if model.player.placement.room_index == room_index {
				anyhow::bail!("Can't delete room containing player");
			}

			transaction.update_world(|world| {
				// TODO(pat.m): maybe find a way to do this that _doesn't_ involve touching every Location in the model

				// Fix player spawn
				if world.player_spawn.room_index > room_index {
					world.player_spawn.room_index = world.player_spawn.room_index.saturating_sub(1);
				}

				// Clear or adjust selection
				if let Some(selected_item) = &mut state.selection {
					// TODO(pat.m): this doesn't really make sense for player spawn
					let selected_room_index = selected_item.room_index(&world);

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
				world.connections.retain(|&(wall_a, wall_b)| {
					wall_a.room_index != room_index && wall_b.room_index != room_index
				});

				// Update all connections with corrected room indices
				for (wall_a, wall_b) in world.connections.iter_mut() {
					if wall_a.room_index > room_index {
						wall_a.room_index -= 1;
					}

					if wall_b.room_index > room_index {
						wall_b.room_index -= 1;
					}
				}

				// Actually remove room
				world.rooms.remove(room_index);

				Ok(())
			})?;

			// Fix player position if we've made it this far
			// TODO(pat.m): BAD
			if transaction.model.player.placement.room_index > room_index {
				transaction.model.player.placement.room_index = transaction.model.player.placement.room_index.saturating_sub(1);
			}
		}

		EditorWorldEditCmd::DisconnectRoom(room_index) => {
			transaction.update_world(|world| {
				world.connections.retain(|&(wall_a, wall_b)| {
					wall_a.room_index != room_index && wall_b.room_index != room_index
				});
				
				Ok(())
			})?;
		}

		EditorWorldEditCmd::ConnectWall(source_wall_id, target_wall_id) => {
			transaction.update_world(|world| {
				// Remove any connections to either the source or target walls
				world.connections.retain(|&(wall_a, wall_b)| {
					wall_a != source_wall_id && wall_b != source_wall_id
					&& wall_a != target_wall_id && wall_b != target_wall_id
				});

				// Connect
				world.connections.push((source_wall_id, target_wall_id));
				
				Ok(())
			})?;
		}

		EditorWorldEditCmd::DisconnectWall(wall_id) => {
			transaction.update_world(|world| {
				world.connections.retain(|&(wall_a, wall_b)| {
					wall_a != wall_id && wall_b != wall_id
				});
				
				Ok(())
			})?;
		}

		EditorWorldEditCmd::SplitWall(wall_id, new_position) => {
			transaction.update_world(|world| {
				let room = world.rooms.get_mut(wall_id.room_index)
					.context("Invalid room index")?;

				let wall = room.walls.get(wall_id.wall_index)
					.context("Invalid wall index")?
					.clone();

				let new_wall_index = wall_id.wall_index + 1;

				// Insert the new wall after the target wall
				room.walls.insert(new_wall_index, wall);
				room.wall_vertices.insert(new_wall_index, new_position);

				// Update all connections with corrected wall ids
				for (wall_a, wall_b) in world.connections.iter_mut() {
					if wall_a.room_index == wall_id.room_index && wall_a.wall_index >= new_wall_index {
						wall_a.wall_index += 1;
					}

					if wall_b.room_index == wall_id.room_index && wall_b.wall_index >= new_wall_index {
						wall_b.wall_index += 1;
					}
				}

				Ok(())
			})?;
		}

		EditorWorldEditCmd::DeleteVertex(vertex_id) => {
			transaction.update_world(|world| {
				let room = world.rooms.get_mut(vertex_id.room_index)
					.context("Invalid room index")?;

				if vertex_id.vertex_index >= room.walls.len() {
					anyhow::bail!("Trying to delete invalid vertex");
				}

				room.walls.remove(vertex_id.vertex_index);
				room.wall_vertices.remove(vertex_id.vertex_index);

				let wall_id = vertex_id.to_wall_id();

				// Remove connections to deleted wall
				world.connections.retain(|&(wall_a, wall_b)| {
					wall_a != wall_id && wall_b != wall_id
				});

				// Update all connections with corrected wall ids
				for (wall_a, wall_b) in world.connections.iter_mut() {
					if wall_a.room_index == vertex_id.room_index && wall_a.wall_index > vertex_id.vertex_index {
						wall_a.wall_index -= 1;
					}

					if wall_b.room_index == vertex_id.room_index && wall_b.wall_index > vertex_id.vertex_index {
						wall_b.wall_index -= 1;
					}
				}

				Ok(())
			})?;
		}


		EditorWorldEditCmd::AddObject(object) => {
			// TODO(pat.m): :(
			transaction.update_world(|world| { world.objects.push(object); Ok(()) })?;
		}

		EditorWorldEditCmd::RemoveObject(object_index) => {
			// TODO(pat.m): :(
			transaction.update_world(|world| { world.objects.remove(object_index); Ok(()) })?;
		}
	}

	Ok(())
}