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

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GlobalVertexId {
	pub room_index: usize,
	pub vertex_index: usize,
}

