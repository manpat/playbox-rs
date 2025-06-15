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



pub fn generate() -> World {
	let geometry = generate_geometry();

	World {
		name: "generated".into(),
		objects: SlotMap::with_key(),
		fog: FogParameters {
			color: Color::grey(0.1),
			start: 0.0,
			distance: 30.0,
			emission: 1.0,
			transparency: 0.5,
		},

		player_spawn: Placement {
			room_id: geometry.first_room(),
			position: Vec2::zero(),
			yaw: 0.0,
		},

		geometry,
	}
}


fn generate_geometry() -> WorldGeometry {
	let mut geometry = WorldGeometry::new();

	let mut rooms = Vec::new();
	let mut verts = Vec::new();

	for _ in 0..200 {
		let big_room = rand::random_bool(3.0 / 50.0);

		generate_room_verts(&mut verts, big_room);

		let room = geometry.insert_room_from_positions(&verts);
		rooms.push(room);

		let room = room.get_mut(&mut geometry);

		room.floor_color = Color::grey(rand::random_range(0.4..=1.0));
		room.ceiling_color = room.floor_color;

		if big_room || rand::random_bool(1.0 / 20.0) {
			room.height = rand::random_range(3.0 ..= 6.0);
		} else {
			room.height = rand::random_range(0.65 ..= 1.5);
		}
	}

	let mut walls = Vec::new();
	for room in rooms.iter() {
		let first_wall = room.first_wall(&geometry);

		let mut wall = first_wall;
		let mut cumulative_length = 0.0;

		'main: loop {
			let wall_length = geometry.wall_length(wall);
			cumulative_length += wall_length;

			if wall_length > 0.4 && cumulative_length > 1.0 {
				walls.push(wall);
				cumulative_length = 0.0;
			}

			wall.move_next(&geometry);
			if wall.next_wall(&geometry) == first_wall {
				break 'main
			}
		}
	}

	walls.shuffle(&mut rand::rng());

	for pair in walls.chunks_exact(2) {
		let &[a, b] = pair else { break };

		geometry.connect_wall(a, b).unwrap();

		let a_len = geometry.wall_length(a);
		let b_len = geometry.wall_length(b);

		let min_len = a_len.min(b_len);
		let a_offset_extent = (a_len - min_len) / 2.0;
		let b_offset_extent = (b_len - min_len) / 2.0;

		let a = a.get_mut(&mut geometry);
		a.vertical_offset = rand::random_range(-0.1 ..= 0.1);
		a.horizontal_offset = rand::random_range(-a_offset_extent ..= a_offset_extent);

		let b = b.get_mut(&mut geometry);
		b.vertical_offset = rand::random_range(-0.1 ..= 0.1);
		b.horizontal_offset = rand::random_range(-b_offset_extent ..= b_offset_extent);
	}

	geometry
}


fn generate_room_verts(verts: &mut Vec<Vec2>, big_room: bool) {
	use std::f32::consts::*;

	verts.clear();

	let mut angle = 0.0f32;

	let radius_range = match big_room {
		false => 0.5 ..= 1.5,
		true => 2.0 ..= 4.0,
	};
	let length_range = match big_room {
		false => 0.2 ..= 1.0,
		true => 0.5 ..= 10.0,
	};

	let radius = rand::random_range(radius_range);

	while angle < TAU {
		let perturbment = rand::random_range(0.9 ..= 1.3);

		verts.push(Vec2::from_angle(-angle) * radius * perturbment);

		let length = rand::random_range(length_range.clone());
		let angle_delta = (length / radius).clamp(0.0, TAU * 0.3);

		angle += angle_delta;
	}
}