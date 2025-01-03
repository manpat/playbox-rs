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
	pub objects: Vec<Object>,

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

			objects: vec![],

			player_spawn: Placement {
				room_id: first_room,
				position: Vec2::zero(),
				yaw: 0.0,
			},

			fog: FogParameters::default(),
		}
	}
}


// TODO(pat.m): would be good to move some of the below into a higher level model that can cache transforms, since
// transforms between connected rooms will always be the same.

pub fn calculate_portal_transform(geometry: &WorldGeometry, from: WallId, to: WallId) -> Mat2x3 {
	let from_wall = &geometry.walls[from];
	let to_wall = &geometry.walls[to];

	let (from_wall_start, from_wall_end) = geometry.wall_vertices(from);
	let (to_wall_start, to_wall_end) = geometry.wall_vertices(to);

	let from_wall_length = (from_wall_end - from_wall_start).length();
	let to_wall_length = (to_wall_end - to_wall_start).length();

	let from_wall_dir = (from_wall_end - from_wall_start) / from_wall_length;
	let to_wall_dir = (to_wall_end - to_wall_start) / to_wall_length;


	let aperture_extent = from_wall_length.min(to_wall_length) / 2.0;

	let from_wall_offset = from_wall.horizontal_offset.clamp(aperture_extent-from_wall_length/2.0, from_wall_length/2.0-aperture_extent);
	let to_wall_offset = to_wall.horizontal_offset.clamp(aperture_extent-to_wall_length/2.0, to_wall_length/2.0-aperture_extent);


	let s = from_wall_dir.wedge(-to_wall_dir);
	let c = from_wall_dir.dot(-to_wall_dir);
	let new_x = Vec2::new(c, -s);
	let new_y = Vec2::new(s, c);

	let from_wall_center = (from_wall_start + from_wall_end) / 2.0 + from_wall_dir * from_wall_offset;
	let to_wall_center = (to_wall_start + to_wall_end) / 2.0 + to_wall_dir * to_wall_offset;
	let rotated_to_wall_center = to_wall_center.x * new_x + to_wall_center.y * new_y;
	let translation = from_wall_center - rotated_to_wall_center;

	Mat2x3::from_columns([
		new_x,
		new_y,
		translation,
	])
}