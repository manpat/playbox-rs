use toybox::prelude::*;
use toybox::input::{ContextID, ActionID, InputSystem};
use toybox::input::raw::{Scancode, MouseButton};

use crate::model::{Player, Camera};

pub struct PlayerController {
	camera_rotate_context: ContextID,
	mouse_action: ActionID,

	forward_action: ActionID,
	back_action: ActionID,
	left_action: ActionID,
	right_action: ActionID,
	shift_action: ActionID,
	rotate_camera_action: ActionID,
}

impl PlayerController {
	pub fn new(input: &mut InputSystem) -> PlayerController {
		let mut movement_context = input.new_context("Player Movement");
		let forward_action = movement_context.new_state("Forward", Scancode::W);
		let back_action = movement_context.new_state("Back", Scancode::S);
		let left_action = movement_context.new_state("Left", Scancode::A);
		let right_action = movement_context.new_state("Right", Scancode::D);
		let shift_action = movement_context.new_state("Sprint", Scancode::LShift);
		let rotate_camera_action = movement_context.new_state("Rotate Camera", MouseButton::Left);
		let movement_context = movement_context.build();

		let mut camera_rotate_context = input.new_context("Camera");
		let mouse_action = camera_rotate_context.new_mouse("Mouse", 1.0);
		let camera_rotate_context = camera_rotate_context.build();

		input.enter_context(movement_context);

		PlayerController {
			camera_rotate_context,
			mouse_action,

			forward_action,
			back_action,
			left_action,
			right_action,
			shift_action,
			rotate_camera_action,
		}
	}

	pub fn update(&self, input: &mut InputSystem, player: &mut Player, camera: &mut Camera) {
		if input.frame_state().entered(self.rotate_camera_action) {
			input.enter_context(self.camera_rotate_context);
		}

		if input.frame_state().left(self.rotate_camera_action) {
			input.leave_context(self.camera_rotate_context);
		}

		let frame_state = input.frame_state();

		if let Some(mouse) = frame_state.mouse(self.mouse_action) {
			player.yaw += mouse.x * 0.5;
			camera.pitch = (camera.pitch + mouse.y as f32 * 0.5).clamp(-PI, PI);
		}

		let camera_yaw_mat = Mat4::rotate_y(player.yaw);

		let move_speed = match frame_state.active(self.shift_action) {
			true => 15.0,
			false => 5.0,
		};

		let player_move_fwd = camera_yaw_mat * Vec3::from_z(-move_speed / 60.0);
		let player_move_right = camera_yaw_mat * Vec3::from_x(move_speed / 60.0);

		if frame_state.active(self.forward_action) { player.position += player_move_fwd }
		if frame_state.active(self.back_action) { player.position -= player_move_fwd }
		if frame_state.active(self.left_action) { player.position -= player_move_right }
		if frame_state.active(self.right_action) { player.position += player_move_right }
	}
}