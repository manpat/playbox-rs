use toybox::prelude::*;

#[derive(Debug)]
pub struct Camera {
	pub control_mode: ControlMode,
	pub position: Vec3,
	pub pitch: f32,
	pub yaw: f32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ControlMode {
	OrbitPlayer,
	FreeFly,
}

impl Camera {
	pub fn new() -> Camera {
		Camera {
			control_mode: ControlMode::OrbitPlayer,
			position: Vec3::zero(),
			pitch: -PI/5.0,
			yaw: 0.0,
		}
	}
}