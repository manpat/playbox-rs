use crate::prelude::*;
use model::{Model, Player, Object, WallId, World, WorldChangedEvent};
use model::world::*;

use std::time::{Instant, Duration};


pub const UNDO_ENTRY_MERGE_WINDOW: Duration = Duration::from_millis(600);


#[derive(Debug)]
pub struct UndoStack {
	groups: Vec<UndoGroup>,
	disabled_change_index: usize,
	last_command_time: Instant,

	message_bus: MessageBus,
}

impl UndoStack {
	pub fn new(message_bus: MessageBus) -> UndoStack {
		UndoStack {
			groups: Vec::new(),
			disabled_change_index: 0,
			last_command_time: Instant::now(),

			message_bus,
		}
	}

	pub fn transaction<'m>(&'m mut self, model: &'m mut Model, message_bus: &'m MessageBus) -> Transaction<'m> {
		Transaction {
			undo_stack: self,
			model,
			message_bus,
			group: UndoGroup::default(),
		}
	}

	pub fn clear(&mut self) {
		self.groups.clear();
		self.disabled_change_index = 0;
	}

	fn push_group(&mut self, group: UndoGroup) {
		let in_merge_window = self.last_command_time.elapsed() < UNDO_ENTRY_MERGE_WINDOW;

		self.last_command_time = Instant::now();

		// If we're not at the head change, truncate the stack
		if self.disabled_change_index < self.groups.len() {
			self.groups.drain(self.disabled_change_index..);
		}

		if let Some(last_group) = self.groups.last_mut()
			&& in_merge_window
			&& last_group.can_merge(&group)
		{
			last_group.merge(group);
		}
		else {
			self.groups.push(group);
		}

		self.disabled_change_index = self.groups.len();
	}

	pub fn can_undo(&self) -> bool {
		self.disabled_change_index > 0
	}

	pub fn can_redo(&self) -> bool {
		self.disabled_change_index < self.groups.len()
	}

	pub fn undo(&mut self, model: &mut Model) {
		if !self.can_undo() {
			return;
		}

		let mut context = UndoContext { model, message_bus: &self.message_bus };

		self.disabled_change_index -= 1;
		self.groups[self.disabled_change_index].undo(&mut context);
	}

	pub fn redo(&mut self, model: &mut Model) {
		if !self.can_redo() {
			return;
		}

		let mut context = UndoContext { model, message_bus: &self.message_bus };

		self.groups[self.disabled_change_index].redo(&mut context);
		self.disabled_change_index += 1;
	}

	pub fn set_index(&mut self, model: &mut Model, index: usize) {
		if self.disabled_change_index == index {
			return;
		}

		// Make sure we can never go out of bounds
		let index = index.min(self.groups.len());

		let mut context = UndoContext { model, message_bus: &self.message_bus };

		// Undo
		while self.disabled_change_index > index {
			self.disabled_change_index -= 1;
			self.groups[self.disabled_change_index].undo(&mut context);
		}

		// Redo
		while self.disabled_change_index < index {
			self.groups[self.disabled_change_index].redo(&mut context);
			self.disabled_change_index += 1;
		}
	}

	pub fn index(&self) -> usize {
		self.disabled_change_index
	}

	pub fn len(&self) -> usize {
		self.groups.len()
	}

	pub fn describe(&self, index: usize) -> &str {
		self.groups.get(index)
			.map_or("<invalid index>", UndoGroup::describe)
			.into()
	}
}


#[derive(Default, Debug)]
pub struct UndoGroup {
	changes: Vec<UndoEntry>,
	description: String,
}

impl UndoGroup {
	pub fn push(&mut self, entry: UndoEntry) {
		if let Some(last_entry) = self.changes.last_mut()
			&& last_entry.can_merge(&entry)
		{
			last_entry.merge(entry);
		}
		else {
			self.changes.push(entry);
		}
	}

	pub fn can_merge(&self, other: &UndoGroup) -> bool {
		// A bit yucky but probably reliable enough for now
		self.description == other.description
	}

	pub fn merge(&mut self, other: UndoGroup) {
		for change in other.changes {
			if let Some(last_change) = self.changes.last_mut()
				&& last_change.can_merge(&change)
			{
				last_change.merge(change);
			}
			else {
				self.changes.push(change);
			}
		}
	}

	pub fn is_empty(&self) -> bool {
		self.changes.is_empty()
	}

	pub fn describe(&self) -> &str {
		&self.description
	}

	fn undo(&self, ctx: &mut UndoContext<'_>) {
		for change in self.changes.iter().rev() {
			change.undo(ctx);
		}
	}

	fn redo(&self, ctx: &mut UndoContext<'_>) {
		for change in self.changes.iter() {
			change.redo(ctx);
		}
	}
}



struct UndoContext<'m> {
	model: &'m mut Model,
	message_bus: &'m MessageBus,
}



#[derive(Debug)]
pub enum UndoEntry {
	UpdateRoom {
		room_id: RoomId,
		before: RoomDef,
		after: RoomDef,
	},

	UpdateWall {
		wall_id: WallId,
		before: WallDef,
		after: WallDef,
	},

	UpdateVertex {
		vertex_id: VertexId,
		before: VertexDef,
		after: VertexDef,
	},

	UpdatePlayer {
		before: Player,
		after: Player,
	},

	UpdateObject {
		object_index: usize,
		before: Object,
		after: Object,
	},

	// TODO(pat.m): yuck - this is way too heavy
	UpdateWorld {
		before: World,
		after: World,
	}
}

impl UndoEntry {
	fn can_merge(&self, other: &UndoEntry) -> bool {
		use UndoEntry::*;

		match (self, other) {
			(UpdateRoom{room_id: left, ..}, UpdateRoom{room_id: right, ..}) => left == right,
			(UpdateWall{wall_id: left, ..}, UpdateWall{wall_id: right, ..}) => left == right,
			(UpdateVertex{vertex_id: left, ..}, UpdateVertex{vertex_id: right, ..}) => left == right,
			(UpdateObject{object_index: left, ..}, UpdateObject{object_index: right, ..}) => left == right,

			(UpdateWorld{..}, UpdateWorld{..}) => true,
			(UpdatePlayer{..}, UpdatePlayer{..}) => true,

			_ => false,
		}
	}

	fn merge(&mut self, other: UndoEntry) {
		use UndoEntry::*;

		match (self, other) {
			(UpdateRoom{after: old_after, ..}, UpdateRoom{after: new_after, ..}) => {
				*old_after = new_after;
			}

			(UpdateWall{after: old_after, ..}, UpdateWall{after: new_after, ..}) => {
				*old_after = new_after;
			}

			(UpdateVertex{after: old_after, ..}, UpdateVertex{after: new_after, ..}) => {
				*old_after = new_after;
			}

			(UpdateObject{after: old_after, ..}, UpdateObject{after: new_after, ..}) => {
				*old_after = new_after;
			}

			(UpdateWorld{after, ..}, UpdateWorld{after: new_after, ..}) => {
				*after = new_after;
			}

			(UpdatePlayer{after, ..}, UpdatePlayer{after: new_after, ..}) => {
				*after = new_after;
			}

			_ => {}
		}
	}

	fn undo(&self, ctx: &mut UndoContext<'_>) {
		use UndoEntry::*;

		match self {
			UpdateRoom{room_id, before, ..} => {
				ctx.model.world.geometry.rooms[*room_id].clone_from(before);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWall{wall_id, before, ..} => {
				ctx.model.world.geometry.walls[*wall_id].clone_from(before);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateVertex{vertex_id, before, ..} => {
				ctx.model.world.geometry.vertices[*vertex_id].clone_from(before);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWorld{before, ..} => {
				ctx.model.world.clone_from(before);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdatePlayer{before, ..} => {
				ctx.model.player.clone_from(before);
			}

			UpdateObject{object_index, before, ..} => {
				ctx.model.world.objects[*object_index].clone_from(before);
			}
		}
	}

	fn redo(&self, ctx: &mut UndoContext<'_>) {
		use UndoEntry::*;

		match self {
			UpdateRoom{room_id, after, ..} => {
				ctx.model.world.geometry.rooms[*room_id].clone_from(after);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWall{wall_id, after, ..} => {
				ctx.model.world.geometry.walls[*wall_id].clone_from(after);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateVertex{vertex_id, after, ..} => {
				ctx.model.world.geometry.vertices[*vertex_id].clone_from(after);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWorld{after, ..} => {
				ctx.model.world.clone_from(after);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdatePlayer{after, ..} => {
				ctx.model.player.clone_from(after);
			}

			UpdateObject{object_index, after, ..} => {
				ctx.model.world.objects[*object_index].clone_from(after);
				// TODO(pat.m): fucky - only here because I'm baking objects into room meshes
				ctx.message_bus.emit(WorldChangedEvent);
			}
		}
	}
}




pub struct Transaction<'m> {
	undo_stack: &'m mut UndoStack,
	model: &'m mut Model,
	message_bus: &'m MessageBus,
	group: UndoGroup,
}

impl Transaction<'_> {
	pub fn describe(&mut self, description: impl Into<String>) {
		self.group.description = description.into();
	}

	pub fn submit(&mut self) {
		if self.group.is_empty() {
			return;
		}

		let group = std::mem::take(&mut self.group);
		group.redo(&mut UndoContext { model: self.model, message_bus: self.message_bus });
		self.undo_stack.push_group(group);
	}

	pub fn model(&self) -> &Model {
		&self.model
	}

	pub fn update_room(&mut self, room_id: RoomId, edit: impl FnOnce(&Model, &mut RoomDef) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let room = self.model.world.geometry.rooms.get(room_id)
			.with_context(|| format!("Trying to edit non-existent room {room_id:?}"))?;

		let before = room.clone();
		let mut after = room.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdateRoom {room_id, before, after});

		Ok(())
	}

	pub fn update_wall(&mut self, wall_id: WallId, edit: impl FnOnce(&Model, &mut WallDef) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let wall = self.model.world.geometry.walls.get(wall_id)
			.with_context(|| format!("Trying to edit non-existent {wall_id:?}"))?;

		let before = wall.clone();
		let mut after = wall.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdateWall {wall_id, before, after});

		Ok(())
	}

	pub fn update_vertex(&mut self, vertex_id: VertexId, edit: impl FnOnce(&Model, &mut VertexDef) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let vertex = self.model.world.geometry.vertices.get(vertex_id)
			.with_context(|| format!("Trying to edit non-existent {vertex_id:?}"))?;

		let before = vertex.clone();
		let mut after = vertex.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdateVertex {vertex_id, before, after});

		Ok(())
	}

	pub fn update_object(&mut self, object_index: usize, edit: impl FnOnce(&Model, &mut Object) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let object = self.model.world.objects.get(object_index)
			.with_context(|| format!("Trying to edit non-existent object #{object_index}"))?;

		let before = object.clone();
		let mut after = object.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdateObject {object_index, before, after});

		Ok(())
	}

	pub fn update_world(&mut self, edit: impl FnOnce(&Model, &mut World) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let before = self.model.world.clone();
		let mut after = self.model.world.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdateWorld {before, after});

		Ok(())
	}

	pub fn update_player(&mut self, edit: impl FnOnce(&Model, &mut Player) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let before = self.model.player.clone();
		let mut after = self.model.player.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdatePlayer {before, after});

		Ok(())
	}
}

