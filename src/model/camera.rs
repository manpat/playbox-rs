use toybox::prelude::*;

pub struct Camera {
	pub position: Vec3,
	pub pitch: f32,
	pub yaw: f32,
}

impl Camera {
	pub fn new() -> Camera {
		Camera {
			position: Vec3::zero(),
			pitch: -PI/5.0,
			yaw: 0.0,
		}
	}
}