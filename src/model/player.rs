use toybox::prelude::*;

pub struct Player {
	pub position: Vec3,
	pub yaw: f32,
}

impl Player {
	pub fn new() -> Player {
		Player {
			position: Vec3::zero(),
			yaw: PI/2.0, // along -z
		}
	}
}