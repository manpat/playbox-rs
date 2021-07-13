use toybox::prelude::*;
use toybox::input::InputSystem;
use toybox::input::raw::Scancode;

use crate::model::{Player, Camera};

const CAMERA_PITCH_LIMIT: (f32, f32) = (-PI/2.0, -PI/16.0);


toybox::declare_input_context! {
	struct PlayerActions "Player Control" {
		state forward { "Forward" [Scancode::W] }
		state back { "Back" [Scancode::S] }
		state left { "Left" [Scancode::A] }
		state right { "Right" [Scancode::D] }
		state shift { "Sprint" [Scancode::LShift] }
		mouse mouse { "Mouse" [1.0] }
	}
}


pub struct PlayerController {
	actions: PlayerActions,
	move_speed: f32,
}

impl PlayerController {
	pub fn new(input: &mut InputSystem) -> PlayerController {
		let actions = PlayerActions::new(input);
		input.enter_context(actions.context_id());

		PlayerController {
			actions,
			move_speed: 0.0,
		}
	}

	pub fn update(&mut self, input: &mut InputSystem, player: &mut Player, camera: &mut Camera) {
		let frame_state = input.frame_state();

		if let Some(mouse) = frame_state.mouse(self.actions.mouse) {
			let (pitch_min, pitch_max) = CAMERA_PITCH_LIMIT;

			camera.yaw += mouse.x * 0.5;
			camera.pitch = (camera.pitch + mouse.y as f32 * 0.5).clamp(pitch_min, pitch_max);
		}

		let move_fwd = Vec3::from_z(-1.0);
		let move_right = Vec3::from_x(1.0);

		let mut move_vector = Vec3::zero();

		if frame_state.active(self.actions.forward) { move_vector += move_fwd }
		if frame_state.active(self.actions.back) { move_vector -= move_fwd }
		if frame_state.active(self.actions.left) { move_vector -= move_right }
		if frame_state.active(self.actions.right) { move_vector += move_right }

		let move_vector_length = move_vector.length();
		if move_vector_length > 0.1 {
			let camera_yaw_mat = Mat3x4::rotate_y(camera.yaw);

			let move_direction = move_vector / move_vector_length;
			let target_move_direction = camera_yaw_mat * move_direction;
			let target_yaw = target_move_direction.to_xz().to_angle() + PI/2.0; // WHY?

			let mut angle_diff = target_yaw - player.yaw;
			if angle_diff > PI {
				angle_diff -= 2.0 * PI;
			} else if angle_diff < -PI {
				angle_diff += 2.0 * PI;
			}
			// TODO(pat.m): make this stable - if player is spinning, stay spinning in that direction

			player.yaw += angle_diff * 3.0 / 60.0;

			let base_move_speed = match frame_state.active(self.actions.shift) {
				true => 15.0,
				false => 5.0,
			};

			self.move_speed += (base_move_speed - self.move_speed) * 4.0 / 60.0;

		} else {
			self.move_speed *= 0.8;
		}

		player.position += Mat3x4::rotate_y(player.yaw) * move_fwd * (self.move_speed / 60.0);
	}
}
