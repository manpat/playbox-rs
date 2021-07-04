use toybox::prelude::*;
use toybox::input::{ContextID, ActionID, InputSystem};
use toybox::input::raw::{Scancode, MouseButton};

use crate::model::{Player, Camera};

const CAMERA_PITCH_LIMIT: (f32, f32) = (-PI/2.0, 0.0);


pub struct PlayerController {
	forward_action: ActionID,
	back_action: ActionID,
	left_action: ActionID,
	right_action: ActionID,
	shift_action: ActionID,
	mouse_action: ActionID,

	move_speed: f32,
}

impl PlayerController {
	pub fn new(input: &mut InputSystem) -> PlayerController {
		let mut movement_context = input.new_context("Player Movement");
		let forward_action = movement_context.new_state("Forward", Scancode::W);
		let back_action = movement_context.new_state("Back", Scancode::S);
		let left_action = movement_context.new_state("Left", Scancode::A);
		let right_action = movement_context.new_state("Right", Scancode::D);
		let shift_action = movement_context.new_state("Sprint", Scancode::LShift);
		let mouse_action = movement_context.new_mouse("Mouse", 1.0);
		let movement_context = movement_context.build();

		input.enter_context(movement_context);

		PlayerController {
			forward_action,
			back_action,
			left_action,
			right_action,
			shift_action,
			mouse_action,

			move_speed: 0.0,
		}
	}

	pub fn update(&mut self, input: &mut InputSystem, player: &mut Player, camera: &mut Camera) {
		let frame_state = input.frame_state();

		if let Some(mouse) = frame_state.mouse(self.mouse_action) {
			let (pitch_min, pitch_max) = CAMERA_PITCH_LIMIT;

			camera.yaw += mouse.x * 0.5;
			camera.pitch = (camera.pitch + mouse.y as f32 * 0.5).clamp(pitch_min, pitch_max);
		}

		let move_fwd = Vec3::from_z(-1.0);
		let move_right = Vec3::from_x(1.0);

		let mut move_vector = Vec3::zero();

		if frame_state.active(self.forward_action) { move_vector += move_fwd }
		if frame_state.active(self.back_action) { move_vector -= move_fwd }
		if frame_state.active(self.left_action) { move_vector -= move_right }
		if frame_state.active(self.right_action) { move_vector += move_right }

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

			player.yaw += angle_diff * 3.0 / 60.0;

			let base_move_speed = match frame_state.active(self.shift_action) {
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
