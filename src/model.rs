use crate::prelude::*;
use std::fmt::{self, Display};

pub mod hud;
pub mod world;
pub mod player;
pub mod progress;
pub mod interactions;
pub mod environment;
pub mod processed_world;

pub use hud::*;
pub use world::*;
pub use player::*;
pub use progress::*;
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
	pub room_id: RoomId,
	pub position: Vec2,
}


#[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Placement {
	pub room_id: RoomId,
	pub position: Vec2,
	pub yaw: f32,
}

impl Placement {
	pub fn location(&self) -> Location {
		Location {
			room_id: self.room_id,
			position: self.position,
		}
	}

	pub fn right(&self) -> Vec2 {
		Vec2::from_angle(self.yaw)
	}

	pub fn forward(&self) -> Vec2 {
		-self.right().perp()
	}
}
