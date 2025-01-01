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
}
