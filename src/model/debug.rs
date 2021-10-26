use toybox::prelude::*;


pub struct Debug {
	pub mouse_pos: Vec2,
	pub active: bool,

	pub srgb_active: bool,
	pub perf_active: bool,
}

impl Debug {
	pub fn new() -> Debug {
		Debug {
			mouse_pos: Vec2::zero(),
			active: false,

			srgb_active: false,
			perf_active: false,
		}
	}
}