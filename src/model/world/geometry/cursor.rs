use crate::prelude::*;
use model::{WorldGeometry, WallId, VertexId, RoomId, RoomDef, WallDef, VertexDef};

/// Room Queries
impl RoomId {
	pub fn is_valid(&self, geometry: &WorldGeometry) -> bool {
		geometry.rooms.contains_key(*self)
	}

	pub fn get<'g>(&self, geometry: &'g WorldGeometry) -> &'g RoomDef {
		&geometry.rooms[*self]
	}

	pub fn get_mut<'g>(&self, geometry: &'g mut WorldGeometry) -> &'g mut RoomDef {
		&mut geometry.rooms[*self]
	}

	pub fn try_get<'g>(&self, geometry: &'g WorldGeometry) -> Option<&'g RoomDef> {
		geometry.rooms.get(*self)
	}

	pub fn try_get_mut<'g>(&self, geometry: &'g mut WorldGeometry) -> Option<&'g mut RoomDef> {
		geometry.rooms.get_mut(*self)
	}

	pub fn first_wall(&self, geometry: &WorldGeometry) -> WallId {
		self.get(geometry).first_wall
	}
}

/// Wall Queries
impl WallId {
	pub fn is_valid(&self, geometry: &WorldGeometry) -> bool {
		geometry.walls.contains_key(*self)
	}

	pub fn get<'g>(&self, geometry: &'g WorldGeometry) -> &'g WallDef {
		&geometry.walls[*self]
	}

	pub fn get_mut<'g>(&self, geometry: &'g mut WorldGeometry) -> &'g mut WallDef {
		&mut geometry.walls[*self]
	}

	pub fn try_get<'g>(&self, geometry: &'g WorldGeometry) -> Option<&'g WallDef> {
		geometry.walls.get(*self)
	}

	pub fn try_get_mut<'g>(&self, geometry: &'g mut WorldGeometry) -> Option<&'g mut WallDef> {
		geometry.walls.get_mut(*self)
	}

	pub fn vertex(&self, geometry: &WorldGeometry) -> VertexId {
		self.get(geometry).source_vertex
	}

	pub fn next_vertex(&self, geometry: &WorldGeometry) -> VertexId {
		self.next_wall(geometry).vertex(geometry)
	}

	pub fn room(&self, geometry: &WorldGeometry) -> RoomId {
		self.get(geometry).room
	}

	pub fn connected_wall(&self, geometry: &WorldGeometry) -> Option<WallId> {
		self.get(geometry).connected_wall
	}

	pub fn connected_room(&self, geometry: &WorldGeometry) -> Option<RoomId> {
		self.connected_wall(geometry)
			.map(|wall_id| wall_id.room(geometry))
	}

	pub fn next_wall(&self, geometry: &WorldGeometry) -> WallId {
		self.get(geometry).next_wall
	}

	pub fn prev_wall(&self, geometry: &WorldGeometry) -> WallId {
		self.get(geometry).prev_wall
	}
}

/// Vertex Queries
impl VertexId {
	pub fn is_valid(&self, geometry: &WorldGeometry) -> bool {
		geometry.vertices.contains_key(*self)
	}

	pub fn get<'g>(&self, geometry: &'g WorldGeometry) -> &'g VertexDef {
		&geometry.vertices[*self]
	}

	pub fn get_mut<'g>(&self, geometry: &'g mut WorldGeometry) -> &'g mut VertexDef {
		&mut geometry.vertices[*self]
	}

	pub fn try_get<'g>(&self, geometry: &'g WorldGeometry) -> Option<&'g VertexDef> {
		geometry.vertices.get(*self)
	}

	pub fn try_get_mut<'g>(&self, geometry: &'g mut WorldGeometry) -> Option<&'g mut VertexDef> {
		geometry.vertices.get_mut(*self)
	}

	pub fn position(&self, geometry: &WorldGeometry) -> Vec2 {
		self.get(geometry).position
	}

	pub fn wall(&self, geometry: &WorldGeometry) -> WallId {
		self.get(geometry).outgoing_wall
	}
}

/// Wall Navigation
impl WallId {
	pub fn move_next(&mut self, geometry: &WorldGeometry) {
		*self = self.next_wall(geometry);
	}

	pub fn move_prev(&mut self, geometry: &WorldGeometry) {
		*self = self.prev_wall(geometry);
	}

	pub fn move_connected(&mut self, geometry: &WorldGeometry) -> bool {
		if let Some(connected) = self.connected_wall(geometry) {
			*self = connected;
			true
		} else {
			false
		}
	}
}
