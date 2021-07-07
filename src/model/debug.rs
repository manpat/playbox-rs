use toybox::prelude::*;


pub struct Debug {
	pub mouse_pos: Vec2,
	pub active: bool,
}

impl Debug {
	pub fn new() -> Debug {
		Debug {
			mouse_pos: Vec2::zero(),
			active: false,
		}
	}
}