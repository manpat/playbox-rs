use crate::prelude::*;
use model::{Model, Room, Wall, WallId, World, WorldChangedEvent};

use std::borrow::Cow;
use std::time::{Instant, Duration};


#[derive(Debug)]
pub struct UndoStack {
	changes: Vec<UndoEntry>,
	disabled_change_index: usize,
	merging_enabled: bool,

	last_command_time: Instant,

	message_bus: MessageBus,
}

impl UndoStack {
	pub fn new(message_bus: MessageBus) -> UndoStack {
		UndoStack {
			changes: Vec::new(),
			disabled_change_index: 0,
			merging_enabled: false,

			last_command_time: Instant::now(),

			message_bus,
		}
	}

	pub fn push(&mut self, entry: impl Into<UndoEntry>) {
		self.last_command_time = Instant::now();

		let entry = entry.into();

		// If we're not at the head change, truncate the stack
		if self.disabled_change_index < self.changes.len() {
			self.changes.drain(self.disabled_change_index..);
		}

		// If we can merge, try and get the last change and merge it if possible
		if self.merging_enabled
			&& let Some(last_entry) = self.changes.last_mut()
			&& last_entry.can_merge(&entry)
		{
			last_entry.merge(entry);
		}
		else {
			self.changes.push(entry);
			self.disabled_change_index = self.changes.len();
		}
	}

	pub fn time_since_last_command(&self) -> Duration {
		self.last_command_time.elapsed()
	}

	pub fn set_merging_enabled(&mut self, enabled: bool) {
		self.merging_enabled = enabled;
	}

	pub fn can_undo(&self) -> bool {
		self.disabled_change_index > 0
	}

	pub fn can_redo(&self) -> bool {
		self.disabled_change_index < self.changes.len()
	}

	pub fn undo(&mut self, model: &mut Model) {
		if !self.can_undo() {
			return;
		}

		let mut context = UndoContext { model, message_bus: &self.message_bus };

		self.disabled_change_index -= 1;
		self.changes[self.disabled_change_index].undo(&mut context);
	}

	pub fn redo(&mut self, model: &mut Model) {
		if !self.can_redo() {
			return;
		}

		let mut context = UndoContext { model, message_bus: &self.message_bus };

		self.changes[self.disabled_change_index].redo(&mut context);
		self.disabled_change_index += 1;
	}

	pub fn set_index(&mut self, model: &mut Model, index: usize) {
		if self.disabled_change_index == index {
			return;
		}

		// Make sure we can never go out of bounds
		let index = index.min(self.changes.len());

		let mut context = UndoContext { model, message_bus: &self.message_bus };

		// Undo
		while self.disabled_change_index > index {
			self.disabled_change_index -= 1;
			self.changes[self.disabled_change_index].undo(&mut context);
		}

		// Redo
		while self.disabled_change_index < index {
			self.changes[self.disabled_change_index].redo(&mut context);
			self.disabled_change_index += 1;
		}
	}

	pub fn index(&self) -> usize {
		self.disabled_change_index
	}

	pub fn len(&self) -> usize {
		self.changes.len()
	}

	pub fn describe(&self, index: usize) -> Cow<'_, str> {
		self.changes.get(index)
			.map_or(Cow::from("<invalid index>"), UndoEntry::describe)
	}

	pub fn transaction<'m>(&'m mut self, model: &'m mut Model, message_bus: &'m MessageBus) -> Transaction<'m> {
		Transaction {
			undo_stack: self,
			model,
			message_bus,
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
		room_index: usize,
		before: Room,
		after: Room,
	},

	UpdateWall {
		wall_id: WallId,
		before: Wall,
		after: Wall,
	},

	// TODO(pat.m): yuck - this is way too heavy
	UpdateWorld {
		before: World,
		after: World,
	}
}

impl UndoEntry {
	fn describe(&self) -> Cow<'_, str> {
		use UndoEntry::*;

		match self {
			UpdateRoom{room_index, ..} => format!("Update room #{room_index}").into(),
			UpdateWall{wall_id: WallId{room_index, wall_index}, ..} => format!("Update wall #{wall_index} in room #{room_index}").into(),
			UpdateWorld{..} => "Update world".into(),
		}
	}

	fn can_merge(&self, other: &UndoEntry) -> bool {
		use UndoEntry::*;

		match (self, other) {
			(UpdateRoom{room_index: left_index, ..}, UpdateRoom{room_index: right_index, ..}) => left_index == right_index,
			(UpdateWall{wall_id: left_id, ..}, UpdateWall{wall_id: right_id, ..}) => left_id == right_id,

			(UpdateRoom{room_index, ..}, UpdateWall{wall_id, ..}) => *room_index == wall_id.room_index,

			(UpdateWorld{..}, UpdateWorld{..}) => true,

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

			(UpdateRoom{after: room, ..}, UpdateWall{after: wall, wall_id: WallId{wall_index, ..}, ..}) => {
				room.walls[wall_index].clone_from(&wall);
			}

			(UpdateWorld{after, ..}, UpdateWorld{after: new_after, ..}) => {
				*after = new_after;
			}

			_ => {}
		}
	}

	fn undo(&self, ctx: &mut UndoContext<'_>) {
		use UndoEntry::*;

		match self {
			UpdateRoom{room_index, before, ..} => {
				ctx.model.world.rooms[*room_index].clone_from(before);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWall{wall_id, before, ..} => {
				ctx.model.world.rooms[wall_id.room_index].walls[wall_id.wall_index].clone_from(before);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWorld{before, ..} => {
				ctx.model.world.clone_from(before);
				ctx.message_bus.emit(WorldChangedEvent);
			}
		}
	}

	fn redo(&self, ctx: &mut UndoContext<'_>) {
		use UndoEntry::*;
		
		match self {
			UpdateRoom{room_index, after, ..} => {
				ctx.model.world.rooms[*room_index].clone_from(after);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWall{wall_id, after, ..} => {
				ctx.model.world.rooms[wall_id.room_index].walls[wall_id.wall_index].clone_from(after);
				ctx.message_bus.emit(WorldChangedEvent);
			}

			UpdateWorld{after, ..} => {
				ctx.model.world.clone_from(after);
				ctx.message_bus.emit(WorldChangedEvent);
			}
		}
	}
}




pub struct Transaction<'m> {
	pub undo_stack: &'m mut UndoStack,
	pub model: &'m mut Model,
	pub message_bus: &'m MessageBus,
}

impl Transaction<'_> {
	pub fn update_room(&mut self, room_index: usize, edit: impl FnOnce(&mut Room) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let room = self.model.world.rooms.get_mut(room_index)
			.with_context(|| format!("Trying to edit non-existent room #{room_index}"))?;

		let before = room.clone();

		if let Err(err) = edit(room) {
			*room = before;
			return Err(err);
		}

		self.undo_stack.push(UndoEntry::UpdateRoom {
			room_index, 
			before,
			after: room.clone(),
		});

		self.message_bus.emit(WorldChangedEvent);

		Ok(())
	}

	pub fn update_wall(&mut self, wall_id: WallId, edit: impl FnOnce(&mut Wall) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let room = self.model.world.rooms.get_mut(wall_id.room_index)
			.with_context(|| format!("Trying to edit wall in non-existent room #{}", wall_id.room_index))?;

		let wall = room.walls.get_mut(wall_id.wall_index)
			.with_context(|| format!("Trying to edit non-existent wall {wall_id:?}"))?;

		let before = wall.clone();

		if let Err(err) = edit(wall) {
			*wall = before;
			return Err(err);
		}

		self.undo_stack.push(UndoEntry::UpdateWall {
			wall_id, 
			before,
			after: wall.clone(),
		});

		self.message_bus.emit(WorldChangedEvent);

		Ok(())
	}

	pub fn update_world(&mut self, edit: impl FnOnce(&mut World) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let before = self.model.world.clone();

		if let Err(err) = edit(&mut self.model.world) {
			self.model.world = before;
			return Err(err);
		}

		self.undo_stack.push(UndoEntry::UpdateWorld {
			before,
			after: self.model.world.clone(),
		});

		self.message_bus.emit(WorldChangedEvent);

		Ok(())
	}
}

