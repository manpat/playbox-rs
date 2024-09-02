use crate::prelude::*;
use model::{WorldPosition, GlobalVertexId, GlobalWallId};

// world is set of rooms, described by walls.
// rooms are connected by wall pairs

pub struct WorldChangedEvent;


#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct World {
	pub rooms: Vec<Room>,
	pub connections: Vec<(GlobalWallId, GlobalWallId)>,

	#[serde(default)]
	pub player_spawn_position: WorldPosition,
	
	#[serde(default)]
	pub player_spawn_yaw: f32,

	pub fog_color: Color,
}

impl World {
	pub fn new() -> World {
		World {
			rooms: vec![Room::new_square(2.0)],
			connections: vec![],

			player_spawn_position: WorldPosition {
				room_index: 0,
				local_position: Vec2::zero(),
			},

			player_spawn_yaw: 0.0,

			fog_color: Color::white(),
		}
	}

	pub fn try_move_by(&self, position: &mut WorldPosition, yaw: Option<&mut f32>, delta: Vec2) {
		if delta.dot(delta) <= 0.00001 {
			return;
		}

		let mover_radius = 0.1;

		let current_room = &self.rooms[position.room_index];
		let mut desired_position = position.local_position + delta;

		fn collide_vertex(desired_position: &mut Vec2, vertex: Vec2, radius: f32) {
			let desired_delta = *desired_position - vertex;
			let penetration = radius - desired_delta.length();
			// TODO(pat.m): this should involve the incoming direction so that a large
			// enough delta can't just pass through the vertex

			if penetration > 0.0 {
				let direction = desired_delta.normalize();
				*desired_position += direction * penetration;
			}
		}

		// Collide with room verts
		for vertex in current_room.wall_vertices.iter() {
			collide_vertex(&mut desired_position, *vertex, mover_radius);
		}

		// Collide with walls
		for wall_index in 0..current_room.walls.len() {
			let (wall_start, wall_end) = current_room.wall_vertices(wall_index);

			let wall_direction = (wall_end - wall_start).normalize();
			let wall_length = (wall_end - wall_start).length();

			let desired_delta_wall_space = desired_position - wall_start;
			let wall_penetration = wall_direction.wedge(desired_delta_wall_space);

			// ASSUME: rooms are convex, and walls are specified in CCW order.

			// Clockwise wedge product means desired position is on the 'inside'
			if wall_penetration + mover_radius < 0.0 {
				continue
			}

			// If the wall ends a long way away then don't continue
			let distance_along_wall = wall_direction.dot(desired_delta_wall_space);
			if distance_along_wall < 0.0 || distance_along_wall >= wall_length {
				continue
			}

			// We have some kind of intersection here - figure out if we need to transition to another room
			// or if we need to slide against the wall
			let wall_id = GlobalWallId{room_index: position.room_index, wall_index};
			if let Some(opposing_wall_id) = self.wall_target(wall_id) {
				// Connected walls may be different lengths, so we need to calculate the aperture that we can actually
				// pass through.
				let opposing_wall_length = {
					let opposing_room = &self.rooms[opposing_wall_id.room_index];
					let (wall_start, wall_end) = opposing_room.wall_vertices(opposing_wall_id.wall_index);
					(wall_end - wall_start).length()
				};

				let apperture_extent = wall_length.min(opposing_wall_length) / 2.0;

				let wall_center = wall_length/2.0;
				let apperture_a = wall_start + (wall_center - apperture_extent) * wall_direction;
				let apperture_b = wall_start + (wall_center + apperture_extent) * wall_direction;
				let intersection_dist_from_center = (wall_center - distance_along_wall).abs();

				// Collide with the virtual apperture verts
				collide_vertex(&mut desired_position, apperture_a, mover_radius);
				collide_vertex(&mut desired_position, apperture_b, mover_radius);

				// If we're transitioning through the aperture then we need to transition to the opposing room.
				// Otherwise just slide as normal.
				if intersection_dist_from_center < apperture_extent {
					if wall_penetration < 0.0 {
						continue
					}

					let transform = calculate_portal_transform(self, opposing_wall_id, wall_id);

					position.room_index = opposing_wall_id.room_index;
					position.local_position = transform * desired_position;

					// Apply yaw offset
					if let Some(yaw) = yaw {
						let row = transform.rows[0];
						let angle_delta = row.y.atan2(row.x);
						*yaw -= angle_delta;
					}

					// TODO(pat.m): collide with walls in opposing wall as well
					return;
				}
			}

			// Slide along wall
			desired_position -= wall_direction.perp() * (wall_penetration + mover_radius);
		}

		// If we get here, no transitions have happened and desired_position has been adjusted to remove wall collisions
		position.local_position = desired_position;
	}

	pub fn vertex(&self, vertex_id: GlobalVertexId) -> Vec2 {
		self.rooms[vertex_id.room_index]
			.wall_vertices[vertex_id.vertex_index]
	}

	pub fn wall_vertices(&self, wall_id: GlobalWallId) -> (Vec2, Vec2) {
		self.rooms[wall_id.room_index]
			.wall_vertices(wall_id.wall_index)
	}

	pub fn wall_center(&self, wall_id: GlobalWallId) -> Vec2 {
		let (start, end) = self.wall_vertices(wall_id);
		(start + end) / 2.0
	}

	pub fn wall_vector(&self, wall_id: GlobalWallId) -> Vec2 {
		let (start, end) = self.wall_vertices(wall_id);
		end - start
	}

	pub fn wall_length(&self, wall_id: GlobalWallId) -> f32 {
		self.wall_vector(wall_id).length()
	}

	pub fn wall_target(&self, wall_id: GlobalWallId) -> Option<GlobalWallId> {
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

	pub fn next_wall(&self, wall_id: GlobalWallId) -> GlobalWallId {
		let num_walls = self.rooms[wall_id.room_index].walls.len();
		GlobalWallId {
			room_index: wall_id.room_index,
			wall_index: (wall_id.wall_index + 1) % num_walls,
		}
	}

	pub fn prev_wall(&self, wall_id: GlobalWallId) -> GlobalWallId {
		let num_walls = self.rooms[wall_id.room_index].walls.len();
		GlobalWallId {
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

			walls: vec![Wall{ color: Color::grey(0.5) }; 4],
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
}




pub fn calculate_portal_transform(world: &World, from: GlobalWallId, to: GlobalWallId) -> Mat2x3 {
	let from_room = &world.rooms[from.room_index];
	let to_room = &world.rooms[to.room_index];

	let (from_wall_start, from_wall_end) = from_room.wall_vertices(from.wall_index);
	let (to_wall_start, to_wall_end) = to_room.wall_vertices(to.wall_index);

	let from_wall_dir = (from_wall_end - from_wall_start).normalize();
	let to_wall_dir = (to_wall_end - to_wall_start).normalize();

	let s = from_wall_dir.wedge(-to_wall_dir);
	let c = from_wall_dir.dot(-to_wall_dir);
	let new_x = Vec2::new(c, -s);
	let new_y = Vec2::new(s, c);

	let from_wall_center = (from_wall_start + from_wall_end) / 2.0;
	let to_wall_center = (to_wall_start + to_wall_end) / 2.0;
	let rotated_to_wall_center = to_wall_center.x * new_x + to_wall_center.y * new_y;
	let translation = from_wall_center - rotated_to_wall_center;

	Mat2x3::from_columns([
		new_x,
		new_y,
		translation,
	])
}


// TODO(pat.m): these should use the resource manager
impl World {
	pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
		let path = path.as_ref();

		if let Some(parent_path) = path.parent() {
			std::fs::create_dir_all(parent_path)?;
		}

		let data = serde_json::to_vec_pretty(self)?;
		std::fs::write(path, &data).map_err(Into::into)
	}

	pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<World> {
		let data = std::fs::read(path)?;
		serde_json::from_slice(&data).map_err(Into::into)
	}
}