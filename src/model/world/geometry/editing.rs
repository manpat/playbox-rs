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
		let new_wall = self.walls.insert(wall_def);

		self.vertices[new_vertex].outgoing_wall = new_wall;

		current_wall.get_mut(self).next_wall = new_wall;
		next_wall.get_mut(self).prev_wall = new_wall;

		new_wall
	}

	pub fn connect_wall(&mut self, wall_id: WallId, new_target: impl Into<Option<WallId>>) {
		let new_target = new_target.into();

		assert!(new_target != Some(wall_id), "Can't connect wall to itself");

		if let Some(old_target) = wall_id.connected_wall(self) {
			if new_target == Some(old_target) {
				// Already connected, early out
				return;
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
	}
}
