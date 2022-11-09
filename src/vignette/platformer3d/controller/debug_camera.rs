use toybox::prelude::*;
use toybox::input::raw::Scancode;

use crate::platformer3d::model::{self, Camera};
use model::camera::ControlMode;

const CAMERA_PITCH_LIMIT: (f32, f32) = (-PI/2.0, PI/2.0);

toybox::declare_input_context! {
	struct DebugCameraActions "Debug Camera Control" {
		state forward { "Forward" [Scancode::W] }
		state back { "Back" [Scancode::S] }
		state left { "Left" [Scancode::A] }
		state right { "Right" [Scancode::D] }
		state shift { "Sprint" [Scancode::LShift] }
		mouse mouse { "Mouse" [1.0] }
	}
}

pub struct DebugCameraController {
	actions: DebugCameraActions,
}


impl DebugCameraController {
	pub fn new(engine: &mut toybox::Engine) -> Self {
		DebugCameraController {
			actions: DebugCameraActions::new_active(engine),
		}
	}

	pub fn update(&mut self, engine: &mut toybox::Engine, camera: &mut Camera) {
		if camera.control_mode != ControlMode::FreeFly {
			if engine.input.is_context_active(self.actions.context_id()) {
				engine.input.leave_context(self.actions.context_id());
			}

			return
		}

		if !engine.input.is_context_active(self.actions.context_id()) {
			engine.input.enter_context(self.actions.context_id());
		}


		let frame_state = engine.input.frame_state();

		if let Some(mouse) = frame_state.mouse(self.actions.mouse) {
			let (pitch_min, pitch_max) = CAMERA_PITCH_LIMIT;

			camera.yaw -= mouse.x * 0.5;
			camera.pitch = (camera.pitch + mouse.y as f32 * 0.5).clamp(pitch_min, pitch_max);
		}

		let camera_orientation = Quat::from_yaw(camera.yaw) * Quat::from_pitch(camera.pitch);
		let mut move_direction = Vec3::zero();

		if frame_state.active(self.actions.forward) { move_direction += camera_orientation.forward() }
		if frame_state.active(self.actions.back) { move_direction -= camera_orientation.forward() }
		if frame_state.active(self.actions.left) { move_direction -= camera_orientation.right() }
		if frame_state.active(self.actions.right) { move_direction += camera_orientation.right() }

		let move_speed = match frame_state.active(self.actions.shift) {
			true => 20.0,
			false => 10.0,
		};

		camera.position += move_speed * move_direction / 60.0;
	}
}