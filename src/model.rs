use crate::prelude::*;
use std::fmt::{self, Display};

pub mod world;
pub mod player;
pub mod interactions;
pub mod environment;
pub mod processed_world;

pub use world::*;
pub use player::*;
pub use interactions::*;
pub use environment::*;
pub use processed_world::*;

#[derive(Debug)]
pub struct Model {
	// Immutable - loaded from resources, edited by editor
	pub world: World,
	// TODO(pat.m): item info
	// TODO(pat.m): any dialogue stuff

	// Mutable - save game state

	pub player: Player,

	// TODO(pat.m): active effects/stats/equipment
	// TODO(pat.m): general behaviours - spell casting/attacks/etc

	// Keeps track of progression. what doors unlocked, items gathered, etc
	pub progress: ProgressModel,


	// Mutable - runtime state, generated from above on load

	// Caches info about World in faster to access way.
	// 	- graph of connections for path finding, traversal
	//  	- + easy distance calculations
	//  - active state of objects - e.g., collected items removed, doors opened/closed
	// 	- active state of rooms/walls - if they can ever be enabled/disabled, heights changed, etc
	//  - fast enumeration of items in a room for rendering
	pub processed_world: ProcessedWorld,

	// Keeps track of what kind of interactions are available, where, and responsible for triggering effects.
	// Derived from ProcessedWorld - interacts with Hud and Player
	pub interactions: Interactions,

	// Keeps track of state of environmental effects - fog settings, reverb settings, active particle effects etc
	pub environment: EnvironmentModel,

	// Everything to do with hud ui - active dialog info, stats display, interactability feedback.
	pub hud: HudModel,
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


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
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

impl Display for WallId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Wall {}:{}", self.room_index, self.wall_index)
	}
}



#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
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

impl Display for VertexId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Vertex {}:{}", self.room_index, self.vertex_index)
	}
}


#[derive(Debug, Default)] pub struct ProgressModel;
#[derive(Debug, Default)] pub struct HudModel;
