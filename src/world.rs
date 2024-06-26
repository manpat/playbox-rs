use toybox::common::*;

// world is set of rooms, described by walls.
// rooms are connected by wall pairs

pub struct World {
	pub rooms: Vec<Room>,
	pub connections: Vec<(GlobalWallId, GlobalWallId)>,
}

impl World {
	pub fn new() -> World {
		World {
			rooms: vec![
				Room {
					walls: [const {Wall{color: Color::light_red()}}; 4].into(),
					wall_vertices: vec![
						Vec2::new(-1.0, -1.0),
						Vec2::new(-1.0,  1.0),
						Vec2::new( 1.0,  1.0),
						Vec2::new( 1.0, -1.0),
					],
				},

				Room {
					walls: [const {Wall{color: Color::light_cyan()}}; 4].into(),
					wall_vertices: vec![
						Vec2::new(-2.0, -1.5),
						Vec2::new(-2.0,  1.0),
						Vec2::new( 0.0,  1.0),
						Vec2::new( 0.0, -1.5),
					],
				},

				Room {
					walls: [const {Wall{color: Color::light_green()}}; 4].into(),
					wall_vertices: vec![
						Vec2::new(0.0, 0.0),
						Vec2::new(0.0, 1.0),
						Vec2::new(4.0, 0.5),
						Vec2::new(4.0, 0.0),
					],
				}
			],

			connections: vec![
				(GlobalWallId{room_index: 0, wall_index: 0}, GlobalWallId{room_index: 1, wall_index: 2}),

				(GlobalWallId{room_index: 0, wall_index: 1}, GlobalWallId{room_index: 1, wall_index: 0}),
				(GlobalWallId{room_index: 0, wall_index: 2}, GlobalWallId{room_index: 2, wall_index: 2}),

				// (GlobalWallId{room_index: 0, wall_index: 3}, GlobalWallId{room_index: 2, wall_index: 1}),
				// (GlobalWallId{room_index: 2, wall_index: 2}, GlobalWallId{room_index: 1, wall_index: 0}),

				(GlobalWallId{room_index: 2, wall_index: 1}, GlobalWallId{room_index: 2, wall_index: 3}),
			],

			// connections: vec![
			// 	(GlobalWallId{room_index: 0, wall_index: 3}, GlobalWallId{room_index: 0, wall_index: 0}),
			// 	// (GlobalWallId{room_index: 1, wall_index: 0}, GlobalWallId{room_index: 0, wall_index: 3}),
			// ],
		}
	}

	pub fn try_move_by(&self, position: &mut WorldPosition, yaw: Option<&mut f32>, delta: Vec2) {
		if delta.dot(delta) <= 0.00001 {
			return;
		}

		let current_room = &self.rooms[position.room_index];
		let mut desired_position = position.local_position + delta;

		for wall_index in 0..current_room.walls.len() {
			let (wall_start, wall_end) = current_room.wall_vertices(wall_index);

			let wall_direction = (wall_end - wall_start).normalize();

			let desired_delta_wall_space = desired_position - wall_start;
			let penetration = wall_direction.wedge(desired_delta_wall_space);

			// ASSUME: rooms are convex, and walls are specified in CCW order.

			// Clockwise wedge product means desired position is on the 'inside'
			if penetration < 0.0 {
				continue
			}

			// We have some kind of intersection here - figure out if we need to transition to another room
			// or if we need to slide against the wall
			let wall_id = GlobalWallId{room_index: position.room_index, wall_index};
			if let Some(opposing_wall_id) = self.connections.iter()
				.filter_map(|&(a, b)| {
					if a == wall_id {
						Some(b)
					} else if b == wall_id {
						Some(a)
					} else {
						None
					}
				})
				.next()
			{
				let transform = calculate_portal_transform(self, opposing_wall_id, wall_id);

				// Need to transition
				position.room_index = opposing_wall_id.room_index;
				position.local_position = transform * desired_position;

				// Apply yaw offset
				if let Some(yaw) = yaw {
					let row = transform.rows[0];
					let angle_delta = row.y.atan2(row.x);
					*yaw -= angle_delta;
				}

				return;
			}

			// Slide along wall
			desired_position -= wall_direction.perp() * penetration;
		}

		// If we get here, no transitions have happened and desired_position has been adjusted to remove wall collisions
		position.local_position = desired_position;
	}
}


pub struct Room {
	pub walls: Vec<Wall>,
	pub wall_vertices: Vec<Vec2>,
}

impl Room {
	pub fn wall_vertices(&self, wall_index: usize) -> (Vec2, Vec2) {
		let end_vertex_idx = (wall_index+1) % self.wall_vertices.len();
		(self.wall_vertices[wall_index], self.wall_vertices[end_vertex_idx])
	}
}

pub struct Wall {
	pub color: Color,
}




pub struct WorldView {

}

impl WorldView {
	pub fn new() -> Self {
		Self {}
	}

	pub fn draw(&self, sprites: &mut super::Sprites, world: &World, pov: WorldPosition) {
		// Draw room you're in
		// then for each wall,
		// 	check if it has a neighbouring room, and if so
		// 	calculate transform between connected walls, and build that room,
		// 	using wall intersection to calculate a frustum to cull by

		let mut drawer = WorldDrawer{sprites, world, vertical_offset: 0.0};
		let initial_transform = Mat2x3::rotate_translate(0.0, -pov.local_position);


		const MAX_DEPTH: i32 = 2;

		let mut room_stack = vec![(pov.room_index, initial_transform, 0)];

		while let Some((room_index, transform, current_depth)) = room_stack.pop() {
			drawer.vertical_offset = -((current_depth-1).max(0) as f32 / 10.0);
			drawer.draw_room(room_index, &transform);

			if current_depth >= MAX_DEPTH {
				continue
			}

			let connections = world.connections.iter()
				.filter_map(|&(left, right)| {
					if left.room_index == room_index {
						Some((left, right))
					} else if right.room_index == room_index {
						Some((right, left))
					} else {
						None
					}
				});

			for (current_wall_id, target_wall_id) in connections {
				let portal_transform = calculate_portal_transform(world, current_wall_id, target_wall_id);
				let total_transform = transform * portal_transform;

				room_stack.push((target_wall_id.room_index, total_transform, current_depth+1));

				// If we connect to the same room then we need to draw again with the inverse transform to make sure both walls get recursed through
				if current_wall_id.room_index == target_wall_id.room_index {
					let portal_transform = calculate_portal_transform(world, target_wall_id, current_wall_id);
					let total_transform = transform * portal_transform;

					room_stack.push((target_wall_id.room_index, total_transform, current_depth+1));
				}
			}
		}

	}
}

fn calculate_portal_transform(world: &World, from: GlobalWallId, to: GlobalWallId) -> Mat2x3 {
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



#[derive(Debug, Copy, Clone, Default)]
pub struct WorldPosition {
	pub room_index: usize,
	pub local_position: Vec2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct GlobalWallId {
	pub room_index: usize,
	pub wall_index: usize,
}


struct WorldDrawer<'a> {
	sprites: &'a mut super::Sprites,
	world: &'a World,
	vertical_offset: f32,
}

impl WorldDrawer<'_> {
	fn draw_room(&mut self, room_index: usize, transform: &Mat2x3) {
		let room = &self.world.rooms[room_index];

		let verts = room.wall_vertices.iter()
			.map(|&v| (*transform * v).to_x0y() + Vec3::from_y(self.vertical_offset));

		// Floor
		self.sprites.add_convex_poly(verts, Color::white());

		// Walls
		for wall_index in 0..room.walls.len() {
			let wall_id = GlobalWallId{room_index, wall_index};
			let is_connection = self.world.connections.iter().any(|&(left, right)| left == wall_id || right == wall_id);

			if !is_connection {
				self.draw_wall(room, wall_index, transform);
			}
		}
	}

	fn draw_wall(&mut self, room: &Room, wall_index: usize, transform: &Mat2x3) {
		let (start_vertex, end_vertex) = room.wall_vertices(wall_index);
		let start_vertex = (*transform * start_vertex).to_x0y() + Vec3::from_y(self.vertical_offset);
		let end_vertex = (*transform * end_vertex).to_x0y() + Vec3::from_y(self.vertical_offset);

		let up = Vec3::from_y(0.2);

		let verts = [
			start_vertex,
			start_vertex + up,
			end_vertex + up,
			end_vertex,
		];

		let wall = &room.walls[wall_index];

		self.sprites.add_convex_poly(verts, wall.color);
	}
}