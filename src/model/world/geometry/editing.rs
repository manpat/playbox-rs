use crate::prelude::*;
use model::*;

/// Editing
impl WorldGeometry {
	pub fn split_wall(&mut self, current_wall: WallId, position: Vec2) -> WallId {
		let new_vertex = self.vertices.insert(VertexDef { position, .. VertexDef::default() });

		let next_wall = current_wall.next_wall(self);

		let mut wall_def = current_wall.get(self).clone();
		wall_def.source_vertex = new_vertex;
		wall_def.connected_wall = None;
		wall_def.prev_wall = current_wall;
		wall_def.next_wall = next_wall;
		wall_def.vertical_offset = 0.0;
		wall_def.horizontal_offset = 0.0;
		let new_wall = self.walls.insert(wall_def);

		self.vertices[new_vertex].outgoing_wall = new_wall;

		current_wall.get_mut(self).next_wall = new_wall;
		next_wall.get_mut(self).prev_wall = new_wall;

		new_wall
	}

	/// Duplicate vertex and create a new wall connecting them.
	pub fn split_vertex(&mut self, vertex_id: VertexId) -> anyhow::Result<WallId> {
		let prev_incoming = vertex_id.wall(self).prev_wall(self);

		if let Some(potentially_prev_outgoing) = prev_incoming.connected_wall(self) {
			// TODO(pat.m): technically I could, but need to figure out the specific behaviour
			anyhow::ensure!(potentially_prev_outgoing.vertex(self) != vertex_id, "Can't split vertex with more than one outgoing wall");
		}

		Ok(self.split_wall(prev_incoming, vertex_id.position(self)))
	}

	/// Delete a vertex and its outgoing wall, if vertex is unique.
	pub fn collapse_vertex(&mut self, vertex_id: VertexId) -> anyhow::Result<()> {
		let wall_id = vertex_id.wall(self);
		let room_id = wall_id.room(self);

		let prev_wall = wall_id.prev_wall(self);
		let next_wall = wall_id.next_wall(self);
		let connected_wall = wall_id.connected_wall(self);

		// Only allow deleting vertices with a single incoming and single outgoing wall
		if let Some(connected_wall) = prev_wall.connected_wall(self)
			&& connected_wall.vertex(self) == vertex_id
		{
			anyhow::bail!("Trying to remove multiply connected vertex")
		}

		if let Some(connected_wall) = connected_wall
			&& connected_wall.next_vertex(self) == vertex_id
		{
			anyhow::bail!("Trying to remove multiply connected vertex")
		}

		// Disconnect wall
		self.connect_wall(wall_id, None)?;

		// Make sure room no longer points to wall.
		let room = room_id.get_mut(self);
		if room.first_wall == wall_id {
			room.first_wall = next_wall;
		}

		// Bridge prev and next walls
		prev_wall.get_mut(self).next_wall = next_wall;
		next_wall.get_mut(self).prev_wall = prev_wall;

		// Finally remove
		self.walls.remove(wall_id);
		self.vertices.remove(vertex_id);

		model::validation::validate_ids(self)?;
		model::validation::validate_room_loop(self, room_id)?;

		Ok(())
	}

	pub fn connect_wall(&mut self, wall_id: WallId, new_target: impl Into<Option<WallId>>) -> anyhow::Result<()> {
		let new_target = new_target.into();

		anyhow::ensure!(new_target != Some(wall_id), "Can't connect wall to itself");

		if let Some(old_target) = wall_id.connected_wall(self) {
			if new_target == Some(old_target) {
				// Already connected, early out
				return Ok(());
			}

			// Disconnect previous target
			old_target.get_mut(self).connected_wall = None;
		}

		if let Some(new_target) = new_target {
			// If our new target was already connected, disconnect it.
			if let Some(old_target) = new_target.connected_wall(self) {
				old_target.get_mut(self).connected_wall = None;
			}

			// Point target to us
			new_target.get_mut(self).connected_wall = Some(wall_id);
		}

		// Point to target
		wall_id.get_mut(self).connected_wall = new_target;

		Ok(())
	}

	/// Splits wall loop off into a new room, connecting it to the original room via `new_loop_start.vertex` and
	/// `new_loop_end.next_vertex`. Returns new wall closing the passed loop.
	// TODO(pat.m): verify that this operation doesn't create any intersections
	pub fn split_room(&mut self, new_loop_start: WallId, new_loop_end: WallId) -> anyhow::Result<WallId> {
		anyhow::ensure!(new_loop_start != new_loop_end, "Trying to create flat room");
		anyhow::ensure!(new_loop_start.room(self) == new_loop_end.room(self), "Trying to split room with walls from different rooms");
		anyhow::ensure!(new_loop_start != new_loop_end.next_wall(self), "Trying to split room with entire room loop");

		let wall_def = new_loop_start.get(self).clone();
		let current_room = new_loop_start.room(self);

		let old_loop_start = new_loop_end.next_wall(self);
		let old_loop_end = new_loop_start.prev_wall(self);

		let current_room_new_wall_vertex = new_loop_start.vertex(self);
		let new_room_new_wall_vertex = old_loop_start.vertex(self);

		let new_wall_current_room = self.walls.insert(WallDef {
			source_vertex: current_room_new_wall_vertex,
			prev_wall: old_loop_end,
			next_wall: old_loop_start,
			room: current_room,
			connected_wall: None,
			.. wall_def.clone()
		});

		old_loop_start.get_mut(self).prev_wall = new_wall_current_room;
		old_loop_end.get_mut(self).next_wall = new_wall_current_room;

		// Set current rooms first wall to our newly created wall, to avoid the case where it was previously
		// one of the split off walls.
		current_room.get_mut(self).first_wall = new_wall_current_room;

		// Split convex 'chunk' into a new room, with same attributes as current room.
		let new_room = self.rooms.insert(current_room.get(self).clone());

		let new_wall_new_room = self.walls.insert(WallDef {
			source_vertex: new_room_new_wall_vertex,
			room: new_room,
			prev_wall: new_loop_end,
			next_wall: new_loop_start,
			connected_wall: None,
			.. wall_def.clone()
		});

		new_loop_start.get_mut(self).prev_wall = new_wall_new_room;
		new_loop_end.get_mut(self).next_wall = new_wall_new_room;
		new_room.get_mut(self).first_wall = new_wall_new_room;

		// Make sure all walls in new room point to it
		{
			let mut wall_it = new_wall_new_room;

			loop {
				wall_it.get_mut(self).room = new_room;
				wall_it.move_next(self);

				if wall_it == new_wall_new_room {
					break
				}
			}
		}

		// Connect new rooms
		new_wall_new_room.get_mut(self).connected_wall = Some(new_wall_current_room);
		new_wall_current_room.get_mut(self).connected_wall = Some(new_wall_new_room);

		Ok(new_wall_new_room)
	}

	/// Ensures the source vertex of a wall is unique.
	/// Returns whether or not a new vertex was created.
	pub fn make_wall_vertex_unique(&mut self, wall_id: WallId) -> anyhow::Result<bool> {
		let vertex_id = wall_id.vertex(self);

		let adjacent_next_wall = wall_id.connected_wall(self).map(|wall| wall.next_wall(self));
		let prev_adjacent_wall = wall_id.prev_wall(self).connected_wall(self);

		let adjacent_next_vertex = adjacent_next_wall.map(|wall| wall.vertex(self));
		let prev_adjacent_vertex = prev_adjacent_wall.map(|wall| wall.vertex(self));

		let adjacent_next_shares_vertex = adjacent_next_vertex == Some(vertex_id);
		let prev_adjacent_shares_vertex = prev_adjacent_vertex == Some(vertex_id);

		// Already unique
		if !adjacent_next_shares_vertex && !prev_adjacent_shares_vertex {
			return Ok(false)
		}

		let vertex_def = vertex_id.get(self).clone();

		if vertex_def.outgoing_wall == wall_id {
			// We need to set the original vertexes outgoing wall to one of the other two wall
			if adjacent_next_shares_vertex {
				vertex_id.get_mut(self).outgoing_wall = adjacent_next_wall.unwrap();
			} else {
				assert!(prev_adjacent_wall.is_some());
				vertex_id.get_mut(self).outgoing_wall = prev_adjacent_wall.unwrap();
			}
		}

		// Create a new vertex and reseat the wall onto it.
		let new_vertex_def = VertexDef {
			outgoing_wall: wall_id,
			.. vertex_def
		};

		let new_vertex_id = self.vertices.insert(new_vertex_def);
		wall_id.get_mut(self).source_vertex = new_vertex_id;

		model::validation::validate_room_loop(self, wall_id.room(self))?;

		Ok(true)
	}
}
