use crate::prelude::*;

#[derive(Debug)]
pub struct Player {
	pub position: model::WorldPosition,
	pub yaw: f32,
	pub pitch: f32,

	pub free_pos: Vec3,
	pub free_cam: bool,
}

impl Player {
	pub fn handle_input(&mut self, ctx: &mut Context<'_>, world: &model::World) {
		if ctx.input.button_just_down(input::keys::KeyV) {
			self.free_cam = !self.free_cam;

			if !self.free_cam {
				self.free_pos = Vec3::zero();
			}
		}

		{
			let (dx, dy) = ctx.input.mouse_delta().map_or((0.0, 0.0), Vec2::to_tuple);
			self.yaw += dx * TAU;
			self.yaw %= TAU;

			let pitch_limit = PI/2.0;
			self.pitch = (self.pitch - 3.0*dy).clamp(-pitch_limit, pitch_limit);
		}

		let speed = match (ctx.input.button_down(input::keys::Shift), ctx.input.button_down(input::keys::Alt)) {
			(true, false) => 4.0 / 60.0,
			(false, true) => 0.5 / 60.0,
			_ => 2.0 / 60.0,
		};

		if self.free_cam {
			// TODO(pat.m): figure out why these need to be negated :(
			// yaw at least I think is because I'm using Vec2::to_x0y, but pitch??
			let yaw_orientation = Quat::from_yaw(-self.yaw);
			let orientation = yaw_orientation * Quat::from_pitch(-self.pitch);

			let right = yaw_orientation.right();
			let forward = orientation.forward();

			if ctx.input.button_down(input::keys::KeyW) {
				self.free_pos += forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyS) {
				self.free_pos -= forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyD) {
				self.free_pos += right * speed;
			}

			if ctx.input.button_down(input::keys::KeyA) {
				self.free_pos -= right * speed;
			}

		} else {
			let right = Vec2::from_angle(self.yaw);
			let forward = -right.perp();

			let mut delta = Vec2::zero();

			if ctx.input.button_down(input::keys::KeyW) {
				delta += forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyS) {
				delta -= forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyD) {
				delta += right * speed;
			}

			if ctx.input.button_down(input::keys::KeyA) {
				delta -= right * speed;
			}

			world.try_move_by(&mut self.position, Some(&mut self.yaw), delta);
		}
	}
}