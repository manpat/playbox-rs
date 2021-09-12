use toybox::prelude::*;
use toybox::input::raw::Scancode;
use toybox::audio::{self, SoundAssetID};

use crate::model::{self, Player, Camera, BlobShadowModel};
use crate::intersect::{Ray, scene_raycast};


toybox::declare_input_context! {
	struct PlayerActions "Player Control" {
		state forward { "Forward" [Scancode::W] }
		state back { "Back" [Scancode::S] }
		state left { "Left" [Scancode::A] }
		state right { "Right" [Scancode::D] }
		state shift { "Sprint" [Scancode::LShift] }
	}
}


pub struct PlayerController {
	actions: PlayerActions,
	move_speed: f32,
	prev_angle_diff: f32,
	next_foot_update: usize,

	footstep_sound_id: SoundAssetID,
	soundbus: audio::BusID,
}

impl PlayerController {
	pub fn new(engine: &mut toybox::Engine) -> PlayerController {
		let actions = PlayerActions::new(&mut engine.input);
		engine.input.enter_context(actions.context_id());

		let footstep_sound_id = {
			let framerate = 44100;
			let freq = 20.0;

			let attack_t = framerate as f32 * 0.001;
			let release_t = framerate as f32 * 0.1;

			let sound_t = attack_t + release_t;
			let buffer_size = sound_t as usize;

			let samples = (0..buffer_size)
				.map(move |x| {
					let x = x as f32;
					let attack = (x / attack_t).min(1.0);
					let release = (1.0 - (x - attack_t) / (sound_t - attack_t)).powf(10.0);

					let envelope = attack*release;

					(x * freq / framerate as f32 * PI).sin() * envelope
				});

			let buffer = audio::Buffer::from_mono_samples(samples);
			engine.audio.register_buffer(buffer)
		};

		let soundbus = engine.audio.new_bus("Player");
		engine.audio.get_bus_mut(soundbus).unwrap().set_gain(0.5);

		PlayerController {
			actions,
			move_speed: 0.0,
			prev_angle_diff: 0.0,
			next_foot_update: 0,
			footstep_sound_id,
			soundbus,
		}
	}

	pub fn update(&mut self, engine: &mut toybox::Engine,
		player: &mut Player, blob_shadows: &mut BlobShadowModel, camera: &Camera, scene: &model::Scene)
	{
		let frame_state = engine.input.frame_state();

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

		let main_scene = scene.main_scene();
		if let Some(hit_pos) = scene_raycast(&main_scene, &ray) {
			player.position.y += (hit_pos.y - player.position.y) / 4.0;
		}

		self.update_feet(engine, player, scene);

		let feet_center = player.feet_positions.iter().sum::<Vec3>() / player.feet_positions.len() as f32;
		let body_height = 1.5;
		let body_target_pos = feet_center + Vec3::from_y(body_height);
		player.body_position += (body_target_pos - player.body_position) * 0.3;
		
		blob_shadows.add(player.body_position, 2.0);

		// if (feet_center-player.position).to_xz().length() > 1.0 {
		// 	player.position += (feet_center - player.position) * 0.08;
		// }
	}

	fn update_feet(&mut self, engine: &mut toybox::Engine, player: &mut Player, scene: &model::Scene) {
		let player_ori = Quat::from_yaw(player.yaw);

		let player_fwd = player_ori.forward();
		let player_right = player_ori.right();

		let feet_center = player.feet_positions.iter().sum::<Vec3>() / player.feet_positions.len() as f32;

		let diff = player.position - feet_center;
		if diff.length() < 1.5 {
			return
		}

		let player_gait = 0.8;

		let feet_offsets = [
			-player_right * player_gait,
			player_right * player_gait,
		];

		let foot_pos = &mut player.feet_positions[self.next_foot_update];
		let foot_offset = feet_offsets[self.next_foot_update];

		let target_pos = player.position + foot_offset;
		let ray = Ray {
			position: target_pos + player_fwd*1.5 + Vec3::from_y(2.0),
			direction: Vec3::from_y(-1.0)
		};

		let scene = scene.main_scene();
		if let Some(hit_pos) = scene_raycast(&scene, &ray) {
			if (hit_pos-feet_center).length() < 6.0 {
				*foot_pos = hit_pos;

				let gain = rand::random::<f32>() * 0.3 + 0.5;
				let bus = engine.audio.get_bus_mut(self.soundbus).unwrap();
				bus.start_sound(self.footstep_sound_id);
				bus.set_gain(gain*gain);
			}
		}

		self.next_foot_update = (self.next_foot_update+1) % player.feet_positions.len();
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