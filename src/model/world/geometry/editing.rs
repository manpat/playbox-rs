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
}
