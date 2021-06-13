use toybox::prelude::*;

pub struct Camera {
	pub zoom: f32,
	pub pitch: f32,
	pub aspect: f32,
}

impl Camera {
	pub fn new() -> Camera {
		Camera {
			zoom: 12.0f32,
			pitch: -PI/5.0,
			aspect: 1.0,
		}
	}
}