use toybox::common::*;



pub struct World {
}

impl World {
	pub fn new() -> World {
		World {
		}
	}

	pub fn update(&mut self) {
	}
}




pub struct WorldView {

}

impl WorldView {
	pub fn new() -> Self {
		Self {}
	}

	pub fn build(&mut self, _world: &World) {

	}

	pub fn draw(&self, sprites: &mut super::Sprites) {
		sprites.basic(Vec3::from_x(4.0), Vec3::from_z(-4.0), Vec3::from_z(2.0), Color::grey(0.5));
	}
}