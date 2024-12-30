use crate::prelude::*;
use slotmap::SlotMap;

slotmap::new_key_type! {
	pub struct VertexId;
	pub struct WallId;
	pub struct RoomId;
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VertexDef {
	pub outgoing_wall: WallId,
	pub position: Vec2i,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WallDef {
	pub source_vertex: VertexId,
	pub next_wall: WallId,
	pub prev_wall: WallId,
	pub connected_wall: Option<WallId>,
	pub room: RoomId,

	pub color: Color,

	// How much to offset the height of the target room.
	pub vertical_offset: i32,

	// How much to offset the aperture horizontally in units from the center of the wall.
	// Clamped to half the length of the wall
	pub horizontal_offset: i32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RoomDef {
	pub first_wall: WallId,

	pub floor_color: Color,
	pub ceiling_color: Color,
	pub height: u32,
}

/// Describes world layout via half-edge structure
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldGeometry {
	pub vertices: SlotMap<VertexId, VertexDef>,
	pub walls: SlotMap<WallId, WallDef>,
	pub rooms: SlotMap<RoomId, RoomDef>,
}


impl WorldGeometry {
	pub fn new() -> WorldGeometry {
		WorldGeometry {
			vertices: SlotMap::with_key(),
			walls: SlotMap::with_key(),
			rooms: SlotMap::with_key(),
		}
	}

	pub fn new_square(size: u32) -> WorldGeometry {
		let mut geometry = WorldGeometry::new();

		let half_size = (size / 2) as i32;

		geometry.insert_room_from_positions(&[
			Vec2i::new(-half_size, -half_size),
			Vec2i::new(-half_size,  half_size),
			Vec2i::new( half_size,  half_size),
			Vec2i::new( half_size, -half_size),
		]);

		geometry
	}

	pub fn insert_room_from_positions(&mut self, positions: &[Vec2i]) -> RoomId {
		assert!(!positions.is_empty());

		let room_id = self.rooms.insert(RoomDef::default());

		let mut wall_ids = Vec::with_capacity(positions.len());

		for &position in positions {
			let vertex_id = self.vertices.insert(VertexDef {
				position,
				.. VertexDef::default()
			});

			let wall_id = self.walls.insert(WallDef {
				source_vertex: vertex_id,
				room: room_id,
				.. WallDef::default()
			});

			self.vertices[vertex_id].outgoing_wall = wall_id;

			wall_ids.push(wall_id);
		}

		let wall_count = wall_ids.len();
		for index in 0..wall_count {
			let next_index = (index + 1) % wall_count;
			let prev_index = (index + wall_count - 1) % wall_count;

			let wall = &mut self.walls[wall_ids[index]];
			wall.next_wall = wall_ids[next_index];
			wall.prev_wall = wall_ids[prev_index];
		}

		self.rooms[room_id].first_wall = wall_ids[0];
		room_id
	}
}

impl WorldGeometry {
	pub fn wall_vertices(&self, wall_id: WallId) -> (Vec2, Vec2) {
		let wall = &self.walls[wall_id];
		let next_wall = &self.walls[wall.next_wall];
		let vertex_0 = self.vertices[wall.source_vertex].position.to_vec2() / 16.0;
		let vertex_1 = self.vertices[next_wall.source_vertex].position.to_vec2() / 16.0;
		(vertex_0, vertex_1)
	}

	pub fn wall_length(&self, wall_id: WallId) -> f32 {
		let (start, end) = self.wall_vertices(wall_id);
		(end - start).length()
	}

	pub fn wall_center(&self, wall_id: WallId) -> Vec2 {
		let (start, end) = self.wall_vertices(wall_id);
		(start + end) / 2.0
	}

	pub fn wall_target(&self, wall_id: WallId) -> Option<WallId> {
		self.walls.get(wall_id)?
			.connected_wall
	}

	pub fn first_room(&self) -> RoomId {
		self.rooms.keys().next().unwrap()
	}

	pub fn room_walls(&self, room_id: RoomId) -> RoomWallIterator<'_> {
		let room = &self.rooms[room_id];
		RoomWallIterator {
			geometry: self,
			first_wall: room.first_wall,
			last_wall: self.walls[room.first_wall].prev_wall,
			fused: false,
		}
	}

	pub fn room_vertices(&self, room_id: RoomId) -> impl Iterator<Item=VertexId> + DoubleEndedIterator + ExactSizeIterator + use<'_> {
		self.room_walls(room_id)
			.map(|wall_id| self.walls[wall_id].source_vertex)
	}

	pub fn room_bounds(&self, room_id: RoomId) -> Aabb2i {
		let mut bounds = Aabb2i::empty();

		for vertex_id in self.room_vertices(room_id) {
			let position = self.vertices[vertex_id].position;
			bounds = bounds.include_point(position);
		}

		bounds
	}
}

#[derive(Clone)]
pub struct RoomWallIterator<'g> {
	geometry: &'g WorldGeometry,
	first_wall: WallId,
	last_wall: WallId,
	fused: bool,
}

impl Iterator for RoomWallIterator<'_> {
	type Item = WallId;

	fn next(&mut self) -> Option<WallId> {
		if self.fused {
			return None
		}

		if self.first_wall == self.last_wall {
			self.fused = true
		}

		let result = self.first_wall;
		self.first_wall = self.geometry.walls[self.first_wall].next_wall;
		Some(result)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		if self.fused {
			return (0, Some(0));
		}

		let mut count = 1;
		let mut it = self.first_wall;

		while it != self.last_wall {
			count += 1;
			it = self.geometry.walls[it].next_wall;
		}

		(count, Some(count))
	}
}

impl DoubleEndedIterator for RoomWallIterator<'_> {
	fn next_back(&mut self) -> Option<WallId> {
		if self.fused {
			return None
		}

		if self.first_wall == self.last_wall {
			self.fused = true
		}

		let result = self.last_wall;
		self.last_wall = self.geometry.walls[self.last_wall].prev_wall;
		Some(result)
	}
}

impl ExactSizeIterator for RoomWallIterator<'_> {}


impl Default for WallDef {
	fn default() -> WallDef {
		WallDef {
			source_vertex: VertexId::default(),
			next_wall: WallId::default(),
			prev_wall: WallId::default(),
			connected_wall: None,
			room: RoomId::default(),

			color: Color::white(),

			vertical_offset: 0,
			horizontal_offset: 0,
		}
	}
}

impl Default for RoomDef {
	fn default() -> RoomDef {
		RoomDef {
			first_wall: WallId::default(),

			floor_color: Color::white(),
			ceiling_color: Color::white(),

			height: 16,
		}
	}
}