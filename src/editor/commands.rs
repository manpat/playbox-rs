use crate::prelude::*;
use model::{WorldChangedEvent, Room, GlobalVertexId, GlobalWallId};
use super::*;


#[derive(Debug)]
pub enum EditorWorldEditCmd {
	TranslateItem(Item, Vec2),

	SetCeilingColor(usize, Color),
	SetFloorColor(usize, Color),
	SetWallColor(GlobalWallId, Color),

	SetFogParams(Color),

	AddRoom {
		room: Room,
		connection: Option<(usize, GlobalWallId)>,
	},

	RemoveRoom(usize),
	DisconnectRoom(usize),

	ConnectWall(GlobalWallId, GlobalWallId),
	DisconnectWall(GlobalWallId),

	SplitWall(GlobalWallId, Vec2),
	DeleteVertex(GlobalVertexId),
}


pub fn handle_editor_cmds(state: &mut State, model: &mut model::Model, message_bus: &MessageBus) {
	let messages_available = message_bus.messages_available(&state.editor_world_edit_cmd_sub);

	for cmd in message_bus.poll_consume(&state.editor_world_edit_cmd_sub) {
		if let Err(err) = handle_world_edit_cmd(state, model, cmd) {
			log::error!("Editor command failed: {err}");
		}
	}

	if messages_available {
		message_bus.emit(WorldChangedEvent);
	}
}


fn handle_world_edit_cmd(state: &mut State, model: &mut model::Model, cmd: EditorWorldEditCmd) -> anyhow::Result<()> {
	if !matches!(cmd, EditorWorldEditCmd::TranslateItem(..)) {
		log::info!("{cmd:?}");
	}

	match cmd {
		EditorWorldEditCmd::TranslateItem(item, delta) => {
			let Some(room) = model.world.rooms.get_mut(item.room_index()) else {
				anyhow::bail!("Trying to edit non-existent room #{}", item.room_index());
			};

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

		EditorWorldEditCmd::SetCeilingColor(room_index, color) => {
			if let Some(room) = model.world.rooms.get_mut(room_index) {
				room.ceiling_color = color;
			} else {
				anyhow::bail!("Trying to edit non-existent room #{room_index}");
			}
		}

		EditorWorldEditCmd::SetFloorColor(room_index, color) => {
			if let Some(room) = model.world.rooms.get_mut(room_index) {
				room.floor_color = color;
			} else {
				anyhow::bail!("Trying to edit non-existent room #{room_index}");
			}
		}

		EditorWorldEditCmd::SetWallColor(wall_id, color) => {
			if let Some(wall) = model.world.rooms.get_mut(wall_id.room_index)
				.and_then(|room| room.walls.get_mut(wall_id.wall_index))
			{
				wall.color = color;
			} else {
				anyhow::bail!("Trying to edit non-existent wall {wall_id:?}");
			}
		}

		EditorWorldEditCmd::SetFogParams(color) => {
			model.world.fog_color = color;
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

				let source_wall_id = GlobalWallId {
					room_index: new_room_index,
					wall_index: source_wall_index,
				};

				// Create new connection
				model.world.connections.push((source_wall_id, target_wall_id));
			}
		}

		EditorWorldEditCmd::RemoveRoom(room_index) => {
			if model.world.rooms.len() == 1 {
				anyhow::bail!("Can't delete last room in world")
			}

			// Fix player position
			if model.player.position.room_index >= room_index {
				if model.player.position.room_index == room_index {
					anyhow::bail!("Can't delete room containing player");
				}

				model.player.position.room_index = model.player.position.room_index.saturating_sub(1);
			}

			// TODO(pat.m): maybe find a way to do this that _doesn't_ involve touching every WorldPosition in the model
			model.world.rooms.remove(room_index);

			// Clear or adjust selection
			if let Some(selected_item) = &mut state.selection {
				let selected_room_index = selected_item.room_index();

				if selected_room_index > room_index {
					selected_item.set_room_index(selected_room_index.saturating_sub(1));
				} else if selected_room_index == room_index {
					state.selection = None;
				}
			}

			// Update focused room
			if state.focused_room_index > room_index {
				state.focused_room_index = state.focused_room_index.saturating_sub(1);
			} else if state.focused_room_index == room_index {
				state.focused_room_index = 0;
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
	}

	Ok(())
}