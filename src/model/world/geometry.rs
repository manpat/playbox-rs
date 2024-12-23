use crate::prelude::*;
use slotmap::Slotmap;

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
	pub vertices: Slotmap<VertexId, VertexDef>,
	pub walls: Slotmap<WallId, WallDef>,
	pub rooms: Slotmap<RoomId, RoomDef>,
}


impl WorldGeometry {
	pub fn new() -> WorldGeometry {
		WorldGeometry {
			vertices: Slotmap::with_key(),
			walls: Slotmap::with_key(),
			rooms: Slotmap::with_key(),
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
				.. VertexDef::default(),
			});

			let wall_id = self.walls.insert(WallDef {
				source_vertex: vertex_id,
				room: room_id,
				.. WallDef::default(),
			});

			self.vertices[vertex_id].outgoing_wall = wall_id;

			wall_ids.push(wall_id);
		}

		let wall_count = wall_ids.len()
		for index in 0..wall_count {
			let next_index = (index + 1) % wall_count;
			let prev_index = (index + wall_count - 1) % wall_count;

			let wall = &mut self.walls[index];
			wall.next_wall = wall_ids[next_index];
			wall.prev_wall = wall_ids[prev_index];
		}

		self.rooms[room_id].first_wall = wall_ids[0];
		room_id
	}
}



impl Default for WallDef {
	fn default() -> RoomDef {
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