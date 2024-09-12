use crate::prelude::*;
use model::{Placement, VertexId, WallId, FogParameters};

mod object;
pub use object::*;

// world is set of rooms, described by walls.
// rooms are connected by wall pairs

#[derive(Clone)]
pub struct WorldChangedEvent;


#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct World {
	// TODO(pat.m): name

	// Describes world layout
	pub rooms: Vec<Room>,
	pub connections: Vec<(WallId, WallId)>,

	pub objects: Vec<Object>,

	pub player_spawn: Placement,
	
	// TODO(pat.m): split out into 'environment settings'
	// TODO(pat.m): can this be specified per room?
	pub fog: FogParameters,
}

impl World {
	pub fn new() -> World {
		World {
			rooms: vec![Room::new_square(2.0)],
			connections: vec![],

			objects: vec![],

			player_spawn: Placement {
				room_index: 0,
				position: Vec2::zero(),
				yaw: 0.0,
			},

			fog: FogParameters::default(),
		}
	}

	pub fn vertex(&self, vertex_id: VertexId) -> Vec2 {
		self.rooms[vertex_id.room_index]
			.wall_vertices[vertex_id.vertex_index]
	}

	pub fn wall_vertices(&self, wall_id: WallId) -> (Vec2, Vec2) {
		self.rooms[wall_id.room_index]
			.wall_vertices(wall_id.wall_index)
	}

	pub fn wall_center(&self, wall_id: WallId) -> Vec2 {
		let (start, end) = self.wall_vertices(wall_id);
		(start + end) / 2.0
	}

	pub fn wall_vector(&self, wall_id: WallId) -> Vec2 {
		let (start, end) = self.wall_vertices(wall_id);
		end - start
	}

	pub fn wall_length(&self, wall_id: WallId) -> f32 {
		self.wall_vector(wall_id).length()
	}

	pub fn wall_target(&self, wall_id: WallId) -> Option<WallId> {
		self.connections.iter()
			.find_map(|&(a, b)| {
				if a == wall_id {
					Some(b)
				} else if b == wall_id {
					Some(a)
				} else {
					None
				}
			})
	}

	pub fn next_wall(&self, wall_id: WallId) -> WallId {
		let num_walls = self.rooms[wall_id.room_index].walls.len();
		WallId {
			room_index: wall_id.room_index,
			wall_index: (wall_id.wall_index + 1) % num_walls,
		}
	}

	pub fn prev_wall(&self, wall_id: WallId) -> WallId {
		let num_walls = self.rooms[wall_id.room_index].walls.len();
		WallId {
			room_index: wall_id.room_index,
			wall_index: (wall_id.wall_index + num_walls - 1) % num_walls,
		}
	}
}


#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Room {
	pub walls: Vec<Wall>,
	pub wall_vertices: Vec<Vec2>,
	pub floor_color: Color,
	pub ceiling_color: Color,
	pub height: f32,
}

impl Room {
	pub fn new_square(wall_length: f32) -> Room {
		let wall_extent = wall_length / 2.0;

		Room {
			wall_vertices: vec![
				Vec2::new( wall_extent, -wall_extent),
				Vec2::new(-wall_extent, -wall_extent),
				Vec2::new(-wall_extent,  wall_extent),
				Vec2::new( wall_extent,  wall_extent),
			],

			walls: vec![Wall::new(); 4],
			floor_color: Color::grey(0.5),
			ceiling_color: Color::grey(0.5),

			height: 1.0,
		}
	}

	pub fn wall_vertices(&self, wall_index: usize) -> (Vec2, Vec2) {
		let end_vertex_idx = (wall_index+1) % self.wall_vertices.len();
		(self.wall_vertices[wall_index], self.wall_vertices[end_vertex_idx])
	}

	pub fn bounds(&self) -> Aabb2 {
		Aabb2::from_points(&self.wall_vertices)
	}
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Wall {
	pub color: Color,

	// How much to offset the height of the target room.
	#[serde(default)]
	pub vertical_offset: f32,

	// How much to offset the aperture horizontally in units from the center of the wall.
	// Clamped to half the length of the wall
	#[serde(default)]
	pub horizontal_offset: f32,
}

impl Wall {
	pub fn new() -> Wall {
		Wall {
			color: Color::white(),
			vertical_offset: 0.0,
			horizontal_offset: 0.0,
		}
	}
}



// TODO(pat.m): would be good to move some of the below into a higher level model that can cache transforms, since
// transforms between connected rooms will always be the same.

pub fn calculate_portal_transform(world: &World, from: WallId, to: WallId) -> Mat2x3 {
	let from_room = &world.rooms[from.room_index];
	let to_room = &world.rooms[to.room_index];

	let from_wall = &from_room.walls[from.wall_index];
	let to_wall = &to_room.walls[to.wall_index];

	let (from_wall_start, from_wall_end) = from_room.wall_vertices(from.wall_index);
	let (to_wall_start, to_wall_end) = to_room.wall_vertices(to.wall_index);

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