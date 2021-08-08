use toybox::prelude::*;

pub struct Player {
	pub position: Vec3,
	pub yaw: f32,

	pub body_position: Vec3,
	pub feet_positions: [Vec3; 2],
}

impl Player {
	pub fn new() -> Player {
		Player {
			position: Vec3::zero(),
			yaw: 0.0, // along -z

			body_position: Vec3::from_y(1.0),
			feet_positions: [Vec3::from_x(-1.0), Vec3::from_x(1.0)],
		}
	}
}