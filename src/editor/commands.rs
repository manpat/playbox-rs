// use crate::prelude::*;
use model::{Model, RoomDef, Object, VertexId, WallId, RoomId, FogParameters};
use super::*;

#[derive(Debug, Clone)]
pub enum EditorModalCmd {
	NewWorld,
	LoadWorld,
	SaveWorld,
	SaveWorldAs,
}


#[derive(Debug)]
pub enum EditorWorldEditCmd {
	TranslateItem(Item, Vec2),

	SetCeilingColor(RoomId, Color),
	SetCeilingHeight(RoomId, u32),
	SetFloorColor(RoomId, Color),

	SetWallColor(WallId, Color),
	SetHorizontalWallOffset(WallId, i32),
	SetVerticalWallOffset(WallId, i32),

	SetFogParams(FogParameters),


	AddRoom {
		room: RoomDef,
		connection: Option<(WallId, WallId)>,
	},

	RemoveRoom(RoomId),
	DisconnectRoom(RoomId),

	ConnectWall(WallId, WallId),
	DisconnectWall(WallId),

	SplitWall(WallId, Vec2i),
	DeleteVertex(VertexId),


	// TODO(pat.m): could be an object
	SetPlayerSpawn,

	AddObject(Object),
	RemoveObject(usize),

	SetObjectName(usize, String),
	EditObject(usize, EditObjectCallback),
}


pub struct EditObjectCallback(Box<dyn FnOnce(&Model, &mut Object) -> anyhow::Result<()> + 'static>);

impl std::fmt::Debug for EditObjectCallback {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "EditObjectCallback")
	}
}

impl EditObjectCallback {
	pub fn new<F>(f: F) -> Self
		where F: FnOnce(&Model, &mut Object) + 'static
	{
		EditObjectCallback(Box::new(move |m, o| {
			f(m, o);
			Ok(())
		}))
	}

	pub fn new_fallible<F>(f: F) -> Self
		where F: FnOnce(&Model, &mut Object) -> anyhow::Result<()> + 'static
	{
		EditObjectCallback(Box::new(f))
	}
}

impl EditorWorldEditCmd {
	pub fn edit_object<F>(object_index: usize, f: F) -> Self
		where F: FnOnce(&Model, &mut Object) + 'static
	{
		EditorWorldEditCmd::EditObject(
			object_index,
			EditObjectCallback::new(f)
		)
	}
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
	}

	drop(transaction);

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

	match cmd {
		// TODO(pat.m): this needs reconsideration
		EditorWorldEditCmd::TranslateItem(item, delta) => {
			let delta_fixed = (delta * 16.0).to_vec2i();

			match item {
				Item::Vertex(vertex_id) => {
					transaction.describe(format!("Move {vertex_id:?}"));
					transaction.update_vertex(vertex_id, |_, vertex| {
						vertex.position += delta_fixed;
						Ok(())
					})?;
					transaction.submit();
				}

				Item::Wall(wall_id) => {
					transaction.describe(format!("Move {wall_id:?}"));

					let geometry = &transaction.model().world.geometry;
					let vertex_a = geometry.walls[wall_id].source_vertex;
					let vertex_b = geometry.walls[geometry.walls[wall_id].next_wall].source_vertex;

					transaction.update_vertex(vertex_a, |_, vertex| {
						vertex.position += delta_fixed;
						Ok(())
					})?;
					transaction.update_vertex(vertex_b, |_, vertex| {
						vertex.position += delta_fixed;
						Ok(())
					})?;
					transaction.submit();
				}

				Item::Room(room_id) => {
					anyhow::bail!("Not implemented");

					// transaction.describe(format!("Recenter Room #{room_id:?}"));
					// transaction.update_room(room_id, |_, room| {
					// 	for vertex in room.wall_vertices.iter_mut() {
					// 		*vertex += delta;
					// 	}

					// 	Ok(())
					// })?;
					// transaction.submit();
				}

				Item::Object(object_index) => {
					transaction.describe(format!("Move Object #{object_index}"));
					transaction.update_object(object_index, |_, object| {
						// TODO(pat.m): need to be able to move between rooms!
						object.placement.position += delta;
						Ok(())
					})?;
					transaction.submit();
				}

				Item::PlayerSpawn => {
					todo!()
				}
			}
		}

		EditorWorldEditCmd::SetCeilingColor(room_id, color) => {
			transaction.describe(format!("Set {room_id:?} ceiling color"));
			transaction.update_room(room_id, |_, room| {
				room.ceiling_color = color;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetCeilingHeight(room_id, height) => {
			transaction.describe(format!("Set {room_id:?} ceiling height"));
			transaction.update_room(room_id, |_, room| {
				room.height = height;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetFloorColor(room_id, color) => {
			transaction.describe(format!("Set {room_id:?} floor color"));
			transaction.update_room(room_id, |_, room| {
				room.floor_color = color;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetWallColor(wall_id, color) => {
			transaction.describe(format!("Set {wall_id:?} color"));
			transaction.update_wall(wall_id, |_, wall| {
				wall.color = color;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetHorizontalWallOffset(wall_id, offset) => {
			transaction.describe(format!("Set {wall_id:?} horizontal offset"));
			transaction.update_wall(wall_id, |_, wall| {
				wall.horizontal_offset = offset;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetVerticalWallOffset(wall_id, offset) => {
			transaction.describe(format!("Set {wall_id:?} vertical offset"));
			transaction.update_wall(wall_id, |_, wall| {
				wall.vertical_offset = offset;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetFogParams(parameters) => {
			transaction.describe("Change fog parameters");
			transaction.update_world(|_, world| {
				world.fog = parameters;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetPlayerSpawn => {
			transaction.describe("Set player spawn");
			transaction.update_world(|model, world| {
				world.player_spawn = model.player.placement;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::AddRoom { room, connection } => {
			anyhow::bail!("Not implemented");

			// if let Some((_, target_wall_id)) = connection {
			// 	transaction.describe(format!("Add Room from {target_wall_id:?}"));
			// } else {
			// 	transaction.describe("Add Room");
			// }

			// transaction.update_world(|_, world| {
			// 	world.rooms.push(room);
			// 	Ok(())
			// })?;

			// if let Some((source_wall_index, target_wall_id)) = connection {
			// 	transaction.update_connections(|model, connections| {
			// 		let new_room_index = model.world.rooms.len();

			// 		// Disconnect target wall
			// 		connections.retain(|&(wall_a, wall_b)| {
			// 			wall_a != target_wall_id && wall_b != target_wall_id
			// 		});

			// 		let source_wall_id = WallId {
			// 			room_index: new_room_index,
			// 			wall_index: source_wall_index,
			// 		};

			// 		// Create new connection
			// 		connections.push((source_wall_id, target_wall_id));

			// 		Ok(())
			// 	})?;
			// }

			// transaction.submit();
		}

		EditorWorldEditCmd::RemoveRoom(room_id) => {
			anyhow::bail!("Not implemented");
			// if transaction.model().world.rooms.len() == 1 {
			// 	anyhow::bail!("Can't delete last room in world")
			// }

			// if transaction.model().player.placement.room_id == room_id {
			// 	anyhow::bail!("Can't delete room containing player");
			// }

			// transaction.describe(format!("Remove Room #{room_id}"));

			// transaction.update_world(|_, world| {
			// 	// TODO(pat.m): maybe find a way to do this that _doesn't_ involve touching every Location in the model

			// 	// Clear or adjust selection
			// 	if let Some(selected_item) = &mut state.selection {
			// 		// TODO(pat.m): this doesn't really make sense for player spawn
			// 		let selected_room_id = selected_item.room_id(&world);

			// 		if selected_room_id == room_id {
			// 			state.selection = None;
			// 		}
			// 	}

			// 	// Actually remove room
			// 	world.geometry.rooms.remove(room_id);

			// 	todo!("Remove walls and vertices");

			// 	Ok(())
			// })?;

			// transaction.submit();
		}

		EditorWorldEditCmd::DisconnectRoom(room_index) => {
			anyhow::bail!("Not implemented");
			// transaction.describe(format!("Disconnect Room #{room_index}"));
			// transaction.update_connections(|_, connections| {
			// 	connections.retain(|&(wall_a, wall_b)| {
			// 		wall_a.room_index != room_index && wall_b.room_index != room_index
			// 	});

			// 	Ok(())
			// })?;
			// transaction.submit();
		}

		EditorWorldEditCmd::ConnectWall(source_wall_id, target_wall_id) => {
			anyhow::bail!("Not implemented");
			// transaction.describe(format!("Connect {source_wall_id} -> {target_wall_id}"));
			// transaction.update_connections(|_, connections| {
			// 	// Remove any connections to either the source or target walls
			// 	connections.retain(|&(wall_a, wall_b)| {
			// 		wall_a != source_wall_id && wall_b != source_wall_id
			// 		&& wall_a != target_wall_id && wall_b != target_wall_id
			// 	});

			// 	// Connect
			// 	connections.push((source_wall_id, target_wall_id));

			// 	Ok(())
			// })?;
			// transaction.submit();
		}

		EditorWorldEditCmd::DisconnectWall(wall_id) => {
			anyhow::bail!("Not implemented");
			// transaction.describe(format!("Disconnect {wall_id:?}"));
			// transaction.update_connections(|_, connections| {
			// 	connections.retain(|&(wall_a, wall_b)| {
			// 		wall_a != wall_id && wall_b != wall_id
			// 	});

			// 	Ok(())
			// })?;
			// transaction.submit();
		}

		EditorWorldEditCmd::SplitWall(wall_id, new_position) => {
			anyhow::bail!("Not implemented");
			// let new_wall_index = wall_id.wall_index + 1;

			// transaction.describe(format!("Split Wall {wall_id:?}"));

			// transaction.update_room(wall_id.room_index, |_, room| {
			// 	let wall = room.walls.get(wall_id.wall_index)
			// 		.context("Invalid wall index")?
			// 		.clone();

			// 	// Insert the new wall after the target wall
			// 	room.walls.insert(new_wall_index, wall);
			// 	room.wall_vertices.insert(new_wall_index, new_position);

			// 	Ok(())
			// })?;

			// transaction.update_connections(|_, connections| {
			// 	// Update all connections with corrected wall ids
			// 	for (wall_a, wall_b) in connections.iter_mut() {
			// 		if wall_a.room_index == wall_id.room_index && wall_a.wall_index >= new_wall_index {
			// 			wall_a.wall_index += 1;
			// 		}

			// 		if wall_b.room_index == wall_id.room_index && wall_b.wall_index >= new_wall_index {
			// 			wall_b.wall_index += 1;
			// 		}
			// 	}

			// 	Ok(())
			// })?;

			// transaction.submit();
		}

		EditorWorldEditCmd::DeleteVertex(vertex_id) => {
			anyhow::bail!("Not implemented");
			// transaction.describe(format!("Remove Vertex {vertex_id}"));

			// // Remove vertex and adjacent wall
			// transaction.update_room(vertex_id.room_index, |_, room| {
			// 	if vertex_id.vertex_index >= room.walls.len() {
			// 		anyhow::bail!("Trying to delete invalid vertex");
			// 	}

			// 	room.walls.remove(vertex_id.vertex_index);
			// 	room.wall_vertices.remove(vertex_id.vertex_index);

			// 	Ok(())
			// })?;

			// // Remove connections to adjacent wall
			// transaction.update_connections(|_, connections| {
			// 	let wall_id = vertex_id.to_wall_id();

			// 	// Remove connections to deleted wall
			// 	connections.retain(|&(wall_a, wall_b)| {
			// 		wall_a != wall_id && wall_b != wall_id
			// 	});

			// 	// Update all connections with corrected wall ids
			// 	for (wall_a, wall_b) in connections.iter_mut() {
			// 		if wall_a.room_index == vertex_id.room_index && wall_a.wall_index > vertex_id.vertex_index {
			// 			wall_a.wall_index -= 1;
			// 		}

			// 		if wall_b.room_index == vertex_id.room_index && wall_b.wall_index > vertex_id.vertex_index {
			// 			wall_b.wall_index -= 1;
			// 		}
			// 	}

			// 	Ok(())
			// })?;

			// transaction.submit();
		}


		EditorWorldEditCmd::AddObject(object) => {
			// TODO(pat.m): :(
			transaction.describe("New Object");
			transaction.update_world(|_, world| { world.objects.push(object); Ok(()) })?;
			transaction.submit();
		}

		EditorWorldEditCmd::RemoveObject(object_index) => {
			// TODO(pat.m): :(
			transaction.describe(format!("Remove Object #{object_index}"));
			transaction.update_world(|_, world| { world.objects.remove(object_index); Ok(()) })?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetObjectName(object_index, object_name) => {
			transaction.describe(format!("Change Object #{object_index}'s name"));
			transaction.update_object(object_index, |_, object| { object.name = object_name; Ok(()) })?;
			transaction.submit();
		}

		EditorWorldEditCmd::EditObject(object_index, func) => {
			transaction.describe(format!("Edit Object #{object_index}"));
			transaction.update_object(object_index, func.0)?;
			transaction.submit();
		}
	}

	Ok(())
}