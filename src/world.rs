use toybox::common::*;

// world is set of rooms, described by walls.
// rooms are connected by wall pairs

pub struct World {
	pub rooms: Vec<Room>, 
}

impl World {
	pub fn new() -> World {
		World {
			rooms: vec![
				Room {
					walls: [const {Wall{color: Color::white()}}; 4].into(),
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
						Vec2::new(-2.0,  1.5),
						Vec2::new(-2.0,  3.0),
						Vec2::new( 2.0,  3.0),
						Vec2::new( 2.0,  1.5),
					],
				}
			]
		}
	}

	pub fn update(&mut self) {
	}
}


pub struct Room {
	pub walls: Vec<Wall>,
	pub wall_vertices: Vec<Vec2>,
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

	// pub fn build(&mut self, _world: &World) {

	// }

	pub fn draw(&self, sprites: &mut super::Sprites, world: &World) {
		// sprites.add(Vec3::from_x(4.0), Vec3::from_z(-4.0), Vec3::from_z(2.0), Color::grey(0.5));

		// Draw room you're in
		// then for each wall,
		// 	check if it has a neighbouring room, and if so
		// 	calculate transform between connected walls, and build that room,
		// 	using wall intersection to calculate a frustum to cull by

		for room in world.rooms.iter() {
			let verts = room.wall_vertices.iter()
				.map(|v| v.to_x0y());

			// Floor
			sprites.add_convex_poly(verts, Color::white());

			// TODO(pat.m): walls/ceil
		}
	}
}