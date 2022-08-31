use toybox::prelude::*;
use toybox::input::raw::Scancode;

use crate::platformer3d::model::{self, Player, Camera};
use model::camera::ControlMode;

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
	pub fn new(engine: &mut toybox::Engine) -> Self {
		CameraController {
			actions: CameraActions::new_active(&mut engine.input),
			zoom: 12.0,
		}
	}

	pub fn update(&mut self, engine: &mut toybox::Engine, camera: &mut Camera, player: &Player) {
		if camera.control_mode != ControlMode::OrbitPlayer {
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

		if frame_state.active(self.actions.zoom_out) {
			self.zoom *= 1.2;
		} else if frame_state.active(self.actions.zoom_in) {
			self.zoom /= 1.2;
		}

		let camera_orientation = Quat::from_yaw(camera.yaw) * Quat::from_pitch(camera.pitch);

		camera.position = player.position + Vec3::from_y(2.0) - camera_orientation.forward() * self.zoom;
	}
}