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
pub struct WorldPosition {
	pub room_index: usize,
	pub local_position: Vec2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GlobalWallId {
	pub room_index: usize,
	pub wall_index: usize,
}

impl GlobalWallId {
	// Each vertex represents the start of a wall, so we can map between their Ids
	pub fn to_vertex_id(&self) -> GlobalVertexId {
		GlobalVertexId {
			room_index: self.room_index,
			vertex_index: self.wall_index,
		}
	}
}



#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GlobalVertexId {
	pub room_index: usize,
	pub vertex_index: usize,
}

impl GlobalVertexId {
	// Each vertex represents the start of a wall, so we can map between their Ids
	pub fn to_wall_id(&self) -> GlobalWallId {
		GlobalWallId {
			room_index: self.room_index,
			wall_index: self.vertex_index,
		}
	}
}
