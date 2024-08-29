use crate::prelude::*;
use model::{WorldChangedEvent, GlobalVertexId, GlobalWallId};
use super::*;


#[derive(Debug)]
pub enum EditorWorldEditCmd {
	TranslateItem(Item, Vec2),

	SetCeilingColor(usize, Color),
	SetFloorColor(usize, Color),
	SetWallColor(GlobalWallId, Color),

	SetFogParams(Color),

	SplitWall(GlobalWallId, Vec2),
	RemoveRoom(usize),
	DisconnectRoom(usize),
	DisconnectWall(GlobalWallId),
}


pub fn handle_editor_cmds(state: &mut State, model: &mut model::Model, message_bus: &MessageBus) {
	let messages = message_bus.poll(&state.editor_world_edit_cmd_sub);

	for cmd in messages.iter() {
		if let Err(err) = handle_world_edit_cmd(model, cmd) {
			log::error!("Editor command failed: {err}");
		}
	}

	if !messages.is_empty() {
		drop(messages);
		message_bus.emit(WorldChangedEvent);
	}
}


fn handle_world_edit_cmd(model: &mut model::Model, cmd: &EditorWorldEditCmd) -> anyhow::Result<()> {
	if !matches!(cmd, EditorWorldEditCmd::TranslateItem(..)) {
		log::info!("{cmd:?}");
	}

	match *cmd {
		EditorWorldEditCmd::TranslateItem(item, delta) => {
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

		EditorWorldEditCmd::SetCeilingColor(room_index, color) => {
			if let Some(room) = model.world.rooms.get_mut(room_index) {
				room.ceiling_color = color;
			}
		}

		EditorWorldEditCmd::SetFloorColor(room_index, color) => {
			if let Some(room) = model.world.rooms.get_mut(room_index) {
				room.floor_color = color;
			}
		}

		EditorWorldEditCmd::SetWallColor(wall_id, color) => {
			if let Some(wall) = model.world.rooms.get_mut(wall_id.room_index)
				.and_then(|room| room.walls.get_mut(wall_id.wall_index))
			{
				wall.color = color;
			}
		}

		EditorWorldEditCmd::SetFogParams(color) => {
			model.world.fog_color = color;
		}

		EditorWorldEditCmd::RemoveRoom(room_index) => {
			// TODO(pat.m): maybe find a way to do this that _doesn't_ involve touching every WorldPosition in the model
			model.world.rooms.remove(room_index);

			// Fix player position
			if model.player.position.room_index >= room_index {
				if model.player.position.room_index == room_index {
					log::warn!("Player in room being deleted!");
				}

				model.player.position.room_index = model.player.position.room_index.saturating_sub(1);
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
	}

	Ok(())
}