use crate::prelude::*;

pub mod world;
pub mod player;

pub use world::*;
pub use player::*;

#[derive(Debug)]
pub struct Model {
	pub world: World,
	pub player: Player,
}



#[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Location {
	pub room_index: usize,
	pub position: Vec2,
}


#[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Placement {
	pub room_index: usize,
	pub position: Vec2,
	pub yaw: f32,
}

impl Placement {
	pub fn location(&self) -> Location {
		Location {
			room_index: self.room_index,
			position: self.position,
		}
	}
}


#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WallId {
	pub room_index: usize,
	pub wall_index: usize,
}

impl WallId {
	// Each vertex represents the start of a wall, so we can map between their Ids
	pub fn to_vertex_id(&self) -> VertexId {
		VertexId {
			room_index: self.room_index,
			vertex_index: self.wall_index,
		}
	}
}



#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VertexId {
	pub room_index: usize,
	pub vertex_index: usize,
}

impl VertexId {
	// Each vertex represents the start of a wall, so we can map between their Ids
	pub fn to_wall_id(&self) -> WallId {
		WallId {
			room_index: self.room_index,
			wall_index: self.vertex_index,
		}
	}
}
