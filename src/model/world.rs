use crate::prelude::*;
use model::{Placement, FogParameters};

mod object;
mod geometry;
pub use object::*;
pub use geometry::*;

// world is set of rooms, described by walls.
// rooms are connected by wall pairs

#[derive(Clone)]
pub struct WorldChangedEvent;

// TODO(pat.m): Turn this into the read-only world definition _resource_
// that only the editor can edit.
// Then process that into the convex-only rooms that we currently have,
// and use that _exclusively_ in other systems.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct World {
	pub name: String,

	#[serde(flatten)]
	pub geometry: WorldGeometry,

	// TODO(pat.m): split out static vs scripted objects
	pub objects: SlotMap<ObjectId, Object>,

	pub player_spawn: Placement,

	// TODO(pat.m): split out into 'environment settings'
	// TODO(pat.m): can this be specified per room?
	pub fog: FogParameters,
}

impl World {
	pub fn new() -> World {
		let geometry = WorldGeometry::new_square(4.0);
		let first_room = geometry.rooms.keys().next().unwrap();

		World {
			name: String::from("default"),

			geometry,
			objects: SlotMap::with_key(),

			player_spawn: Placement {
				room_id: first_room,
				position: Vec2::zero(),
				yaw: 0.0,
			},

			fog: FogParameters::default(),
		}
	}
}
