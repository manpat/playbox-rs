// use crate::prelude::*;
use model::{SourceModel, Placement, RoomDef, Object, ObjectId, VertexId, WallId, RoomId, FogParameters};
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
	SetCeilingHeight(RoomId, f32),
	SetFloorColor(RoomId, Color),

	SetWallColor(WallId, Color),
	SetHorizontalWallOffset(WallId, f32),
	SetVerticalWallOffset(WallId, f32),

	SetFogParams(FogParameters),

	RemoveRoom(RoomId),
	DisconnectRoom(RoomId),

	SplitRoom(WallId, WallId),

	ConnectWall(WallId, WallId),
	DisconnectWall(WallId),

	SplitWall(WallId, Vec2),
	SplitVertex(VertexId),
	DeleteVertex(VertexId),


	// TODO(pat.m): could be an object
	SetPlayerSpawn(Placement),

	AddObject(Object),
	RemoveObject(ObjectId),

	SetObjectName(ObjectId, String),
	EditObject(ObjectId, EditObjectCallback),
}


pub struct EditObjectCallback(Box<dyn FnOnce(&SourceModel, &mut Object) -> anyhow::Result<()> + 'static>);

impl std::fmt::Debug for EditObjectCallback {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "EditObjectCallback")
	}
}

impl EditObjectCallback {
	pub fn new<F>(f: F) -> Self
		where F: FnOnce(&SourceModel, &mut Object) + 'static
	{
		EditObjectCallback(Box::new(move |m, o| {
			f(m, o);
			Ok(())
		}))
	}

	pub fn new_fallible<F>(f: F) -> Self
		where F: FnOnce(&SourceModel, &mut Object) -> anyhow::Result<()> + 'static
	{
		EditObjectCallback(Box::new(f))
	}
}

impl EditorWorldEditCmd {
	pub fn edit_object<F>(object_id: ObjectId, f: F) -> Self
		where F: FnOnce(&SourceModel, &mut Object) + 'static
	{
		EditorWorldEditCmd::EditObject(
			object_id,
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


pub fn handle_editor_cmds(state: &mut State, source_model: &mut model::SourceModel, message_bus: &MessageBus) {
	// Handle undo/redo
	for cmd in message_bus.poll(&state.undo_cmd_sub) {
		if let Err(err) = handle_undo_cmd(&mut state.undo_stack, source_model, cmd) {
			log::error!("{cmd:?} failed: {err}");
		}
	}

	// Handle editor commands
	let mut transaction = state.undo_stack.transaction(source_model, message_bus);

	for cmd in message_bus.poll_consume(&state.editor_world_edit_cmd_sub) {
		if let Err(err) = handle_world_edit_cmd(&mut state.inner, &mut transaction, cmd) {
			log::error!("Editor command failed: {err}");
		}
	}

	drop(transaction);

	// Make sure we haven't messed anything up
	super::validate_model(state, source_model);
}


fn handle_undo_cmd(undo_stack: &mut UndoStack, model: &mut model::SourceModel, cmd: UndoCmd) -> anyhow::Result<()> {
	log::info!("{cmd:?}");

	match cmd {
		UndoCmd::Undo => undo_stack.undo(model),
		UndoCmd::Redo => undo_stack.redo(model),
		UndoCmd::SetIndex(index) => undo_stack.set_index(model, index),
	}

	Ok(())
}

fn handle_world_edit_cmd(_state: &mut InnerState, transaction: &mut Transaction<'_>, cmd: EditorWorldEditCmd) -> anyhow::Result<()> {
	if !matches!(cmd, EditorWorldEditCmd::TranslateItem(..)) {
		log::info!("{cmd:?}");
	}

	match cmd {
		// TODO(pat.m): this needs reconsideration
		EditorWorldEditCmd::TranslateItem(item, delta) => {
			match item {
				Item::Vertex(vertex_id) => {
					transaction.describe(format!("Move {vertex_id:?}"));
					transaction.update_vertex(vertex_id, |_, vertex| {
						vertex.position += delta;
						Ok(())
					})?;
					transaction.submit();
				}

				Item::Wall(wall_id) => {
					transaction.describe(format!("Move {wall_id:?}"));

					let geometry = &transaction.model().world.geometry;
					let vertex_a = wall_id.vertex(geometry);
					let vertex_b = wall_id.next_vertex(geometry);

					transaction.update_vertex(vertex_a, |_, vertex| {
						vertex.position += delta;
						Ok(())
					})?;
					transaction.update_vertex(vertex_b, |_, vertex| {
						vertex.position += delta;
						Ok(())
					})?;
					transaction.submit();
				}

				Item::Room(room_id) => {
					transaction.describe(format!("Move Room #{room_id:?}"));

					let vertices: SmallVec<[VertexId; 8]> = transaction.model().world.geometry
						.room_vertices(room_id)
						.collect();

					for vertex_id in vertices {
						transaction.update_vertex(vertex_id, |_, vertex| {
							vertex.position += delta;
							Ok(())
						})?;
					}

					transaction.submit();
				}

				Item::Object(object_id) => {
					transaction.describe(format!("Move {object_id:?}"));
					transaction.update_object(object_id, |_, object| {
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

		EditorWorldEditCmd::SetPlayerSpawn(placement) => {
			transaction.describe("Set player spawn");
			transaction.update_world(move |_model, world| {
				world.player_spawn = placement;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::RemoveRoom(room_id) => {
			if transaction.model().world.geometry.rooms.len() == 1 {
				anyhow::bail!("Can't delete last room in world")
			}

			transaction.describe(format!("Remove {room_id:?}"));
			transaction.update_geometry(|model, geometry| {
				anyhow::ensure!(room_id.is_valid(geometry));

				let old_geometry = &model.world.geometry;

				for wall_id in old_geometry.room_walls(room_id) {
					if let Some(wall) = wall_id.connected_wall(old_geometry)
						.and_then(|wall_id| wall_id.try_get_mut(geometry))
					{
						wall.connected_wall = None;
					}

					// TODO(pat.m): assert that vertex is unique

					geometry.vertices.remove(wall_id.vertex(old_geometry));
					geometry.walls.remove(wall_id);
				}

				geometry.rooms.remove(room_id);

				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::DisconnectRoom(room_id) => {
			transaction.describe(format!("Disconnect {room_id:?}"));
			transaction.update_geometry(|model, geometry| {
				for wall in model.world.geometry.room_walls(room_id) {
					geometry.connect_wall(wall, None)?;
				}
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SplitRoom(source_wall_id, target_wall_id) => {
			let geometry = &transaction.model().world.geometry;

			anyhow::ensure!(source_wall_id != target_wall_id);
			anyhow::ensure!(source_wall_id.is_valid(geometry));
			anyhow::ensure!(target_wall_id.is_valid(geometry));

			let room = source_wall_id.room(geometry);

			transaction.describe(format!("Split {room:?} by {source_wall_id:?} -> {target_wall_id:?}"));
			transaction.update_geometry(|_, geometry| {
				let new_loop_connecting_wall = geometry.split_room(source_wall_id, target_wall_id)?;

				// Make sure walls in the source model all have unique vertices.
				geometry.make_wall_vertex_unique(new_loop_connecting_wall)?;
				geometry.make_wall_vertex_unique(new_loop_connecting_wall.next_wall(geometry))?;
				Ok(())
			})?;

			// TODO(pat.m): make sure objects in original room end up in the correct room!

			transaction.submit();
		}

		EditorWorldEditCmd::ConnectWall(source_wall_id, target_wall_id) => {
			anyhow::ensure!(source_wall_id != target_wall_id, "Trying to connect wall to itself");
			anyhow::ensure!(source_wall_id.is_valid(&transaction.model().world.geometry));
			anyhow::ensure!(target_wall_id.is_valid(&transaction.model().world.geometry));

			transaction.describe(format!("Connect {source_wall_id:?} -> {target_wall_id:?}"));
			transaction.update_geometry(|_, geometry| {
				geometry.connect_wall(source_wall_id, target_wall_id)?;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::DisconnectWall(wall_id) => {
			anyhow::ensure!(wall_id.is_valid(&transaction.model().world.geometry));

			transaction.describe(format!("Disconnect {wall_id:?}"));
			transaction.update_geometry(|_, geometry| {
				geometry.connect_wall(wall_id, None)?;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SplitWall(wall_id, new_position) => {
			transaction.describe(format!("Split {wall_id:?}"));
			transaction.update_geometry(|_, geometry| {
				geometry.split_wall(wall_id, new_position);
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::SplitVertex(vertex_id) => {
			transaction.describe(format!("Split {vertex_id:?}"));
			transaction.update_geometry(|_, geometry| {
				geometry.split_vertex(vertex_id)?;
				Ok(())
			})?;
			transaction.submit();
		}

		EditorWorldEditCmd::DeleteVertex(vertex_id) => {
			transaction.describe(format!("Remove Vertex {vertex_id:?}"));

			transaction.update_geometry(|_, geometry| {
				geometry.collapse_vertex(vertex_id)?;
				Ok(())
			})?;

			transaction.submit();
		}


		EditorWorldEditCmd::AddObject(object) => {
			// TODO(pat.m): :(
			transaction.describe("New Object");
			transaction.update_world(|_, world| { world.objects.insert(object); Ok(()) })?;
			transaction.submit();
		}

		EditorWorldEditCmd::RemoveObject(object_id) => {
			// TODO(pat.m): :(
			transaction.describe(format!("Remove {object_id:?}"));
			transaction.update_world(|_, world| { world.objects.remove(object_id); Ok(()) })?;
			transaction.submit();
		}

		EditorWorldEditCmd::SetObjectName(object_id, object_name) => {
			transaction.describe(format!("Change {object_id:?}'s name"));
			transaction.update_object(object_id, |_, object| { object.name = object_name; Ok(()) })?;
			transaction.submit();
		}

		EditorWorldEditCmd::EditObject(object_id, func) => {
			transaction.describe(format!("Edit {object_id:?}"));
			transaction.update_object(object_id, func.0)?;
			transaction.submit();
		}
	}

	Ok(())
}