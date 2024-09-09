use crate::prelude::*;
use model::{Model, Player, Room, Wall, WallId, World, WorldChangedEvent};

use std::time::{Instant, Duration};


pub const UNDO_ENTRY_MERGE_WINDOW: Duration = Duration::from_millis(400);


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
	pub fn new(description: impl Into<String>) -> UndoGroup {
		UndoGroup {
			changes: Vec::with_capacity(1),
			description: description.into(),
		}
	}

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
		room_index: usize,
		before: Room,
		after: Room,
	},

	UpdateWall {
		wall_id: WallId,
		before: Wall,
		after: Wall,
	},

	UpdatePlayer {
		before: Player,
		after: Player,
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
			(UpdateRoom{room_index: left_index, ..}, UpdateRoom{room_index: right_index, ..}) => left_index == right_index,
			(UpdateWall{wall_id: left_id, ..}, UpdateWall{wall_id: right_id, ..}) => left_id == right_id,

			(UpdateRoom{room_index, ..}, UpdateWall{wall_id, ..}) => *room_index == wall_id.room_index,

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

			(UpdateRoom{after: room, ..}, UpdateWall{after: wall, wall_id: WallId{wall_index, ..}, ..}) => {
				room.walls[wall_index].clone_from(&wall);
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

			UpdatePlayer{before, ..} => {
				ctx.model.player.clone_from(before);
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

			UpdatePlayer{after, ..} => {
				ctx.model.player.clone_from(after);
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

	pub fn update_room(&mut self, room_index: usize, edit: impl FnOnce(&Model, &mut Room) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let room = self.model.world.rooms.get_mut(room_index)
			.with_context(|| format!("Trying to edit non-existent room #{room_index}"))?;

		let before = room.clone();
		let mut after = room.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdateRoom {
			room_index, 
			before,
			after,
		});

		Ok(())
	}

	pub fn update_wall(&mut self, wall_id: WallId, edit: impl FnOnce(&Model, &mut Wall) -> anyhow::Result<()>) -> anyhow::Result<()> {
		let room = self.model.world.rooms.get_mut(wall_id.room_index)
			.with_context(|| format!("Trying to edit {wall_id} in non-existent room"))?;

		let wall = room.walls.get_mut(wall_id.wall_index)
			.with_context(|| format!("Trying to edit non-existent {wall_id}"))?;

		let before = wall.clone();
		let mut after = wall.clone();

		edit(&self.model, &mut after)?;

		self.group.push(UndoEntry::UpdateWall {
			wall_id, 
			before,
			after,
		});

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


impl<'m> Drop for Transaction<'m> {
	fn drop(&mut self) {
		self.submit();
	}
}
