use toybox::prelude::*;
use toybox::input::InputSystem;
use toybox::input::raw::Scancode;

use crate::model::{self, Player, Camera};
use crate::intersect::{Ray, scene_raycast};

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
	prev_angle_diff: f32,
}

impl PlayerController {
	pub fn new(input: &mut InputSystem) -> PlayerController {
		let actions = PlayerActions::new(input);
		input.enter_context(actions.context_id());

		PlayerController {
			actions,
			move_speed: 0.0,
			prev_angle_diff: 0.0,
		}
	}

	pub fn update(&mut self, input: &mut InputSystem, player: &mut Player, camera: &mut Camera, scene: &model::Scene) {
		let frame_state = input.frame_state();

		if let Some(mouse) = frame_state.mouse(self.actions.mouse) {
			let (pitch_min, pitch_max) = CAMERA_PITCH_LIMIT;

			camera.yaw -= mouse.x * 0.5;
			camera.pitch = (camera.pitch + mouse.y as f32 * 0.5).clamp(pitch_min, pitch_max);
		}

		let camera_orientation = Quat::from_yaw(camera.yaw);
		let mut move_direction = Vec3::zero();

		if frame_state.active(self.actions.forward) { move_direction += camera_orientation.forward() }
		if frame_state.active(self.actions.back) { move_direction -= camera_orientation.forward() }
		if frame_state.active(self.actions.left) { move_direction -= camera_orientation.right() }
		if frame_state.active(self.actions.right) { move_direction += camera_orientation.right() }

		if move_direction.length() > 0.1 {
			let Vec3{x: target_x, z: target_z, ..} = move_direction;
			let target_yaw = (-target_z).atan2(target_x) - PI/2.0;

			let mut angle_diff = angle_difference(target_yaw, player.yaw);
			let angle_diff_2 = angle_diff - self.prev_angle_diff;

			// Make sure rotation is stable - smooth out second order derivative
			if angle_diff_2 > PI {
				angle_diff -= TAU;
			} else if angle_diff_2 < -PI {
				angle_diff += TAU;
			}

			player.yaw += angle_diff * 4.0 / 60.0;

			let base_move_speed = match frame_state.active(self.actions.shift) {
				true => 18.0,
				false => 10.0,
			};

			self.move_speed += (base_move_speed - self.move_speed) * 4.0 / 60.0;
			self.prev_angle_diff = angle_diff;

		} else {
			self.move_speed *= 0.8;
			self.prev_angle_diff = 0.0;
		}

		player.position += Quat::from_yaw(player.yaw).forward() * (self.move_speed / 60.0);

		let ray = Ray {
			position: player.position + Vec3::from_y(2.0),
			direction: Vec3::from_y(-1.0)
		};

		let scene = scene.main_scene();
		if let Some(hit_pos) = scene_raycast(&scene, &ray) {
			player.position.y += (hit_pos.y - player.position.y) / 4.0;
		}
	}
}


fn angle_difference(a: f32, b: f32) -> f32 {
	let mut angle_diff = (a - b) % TAU;

	if angle_diff > PI {
		angle_diff -= TAU;
	} else if angle_diff < -PI {
		angle_diff += TAU;
	}

	angle_diff
}