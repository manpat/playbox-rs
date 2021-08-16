use toybox::prelude::*;
use toybox::input::InputSystem;
use toybox::input::raw::Scancode;

use crate::model::{self, Player, Camera};

const CAMERA_PITCH_LIMIT: (f32, f32) = (-PI/2.0, -PI/16.0);

toybox::declare_input_context! {
	struct CameraActions "Camera Control" {
		trigger zoom_out { "Zoom Out" [Scancode::Minus] }
		trigger zoom_in { "Zoom In" [Scancode::Equals] }
		mouse mouse { "Mouse" [1.0] }
	}
}

pub struct CameraController {
	actions: CameraActions,
	zoom: f32,
}


impl CameraController {
	pub fn new(input: &mut InputSystem) -> Self {
		let actions = CameraActions::new(input);
		input.enter_context(actions.context_id());

		CameraController {
			actions,
			zoom: 12.0,
		}
	}

	pub fn update(&mut self, input: &InputSystem, camera: &mut Camera, player: &Player) {
		let frame_state = input.frame_state();

		if let Some(mouse) = frame_state.mouse(self.actions.mouse) {
			let (pitch_min, pitch_max) = CAMERA_PITCH_LIMIT;

			camera.yaw -= mouse.x * 0.5;
			camera.pitch = (camera.pitch + mouse.y as f32 * 0.5).clamp(pitch_min, pitch_max);
		}

		if frame_state.active(self.actions.zoom_out) {
			self.zoom *= 1.2;
		} else if frame_state.active(self.actions.zoom_in) {
			self.zoom /= 1.2;
		}

		let camera_orientation = Quat::from_yaw(camera.yaw) * Quat::from_pitch(camera.pitch);

		camera.position = player.position + Vec3::from_y(2.0) - camera_orientation.forward() * self.zoom;
	}
}