use crate::prelude::*;
use model::{Placement, WallId, World, ProcessedWorld, HudModel};

/// Ratio of player height to max step distance.
pub const PLAYER_MAX_STEP_HEIGHT_PERCENTAGE: f32 = 0.5;
pub const PLAYER_HEIGHT: f32 = 0.5;
pub const PLAYER_RADIUS: f32 = 0.1;


#[derive(Debug, Clone, PartialEq)]
pub enum PlayerCmd {
	Interact,
}


// TODO(pat.m): maybe this should just be 'camera'? only some of this should be serialized
#[derive(Debug, Clone)]
pub struct Player {
	pub placement: Placement,
	pub pitch: f32,

	pub blood: u32,
	pub salt: u32,

	// TODO(pat.m): these should be separate to player
	pub free_pos: Vec3,
	pub free_cam: bool,

	// TODO(pat.m): yucky :((((
	pub hack_height_change: Option<f32>,
}

impl Player {
	pub fn handle_input(&mut self, ctx: &mut Context<'_>, world: &World, processed_world: &ProcessedWorld, hud: &HudModel) {
		self.hack_height_change = None;

		let interact_pressed = ctx.input.button_just_down(input::MouseButton::Left) || ctx.input.button_just_down(input::keys::KeyF);

		if hud.in_dialog {
			if interact_pressed {
				ctx.message_bus.emit(model::HudCmd::DismissDialog);
			}

			return;
		}

		if ctx.input.button_just_down(input::keys::KeyV) {
			self.free_cam = !self.free_cam;

			if !self.free_cam {
				self.free_pos = Vec3::zero();
			}
		}

		if interact_pressed {
			ctx.message_bus.emit(PlayerCmd::Interact);
		}

		let forward_pressed = ctx.input.button_down(input::keys::KeyW);
		let back_pressed = ctx.input.button_down(input::keys::KeyS);
		let right_pressed = ctx.input.button_down(input::keys::KeyD);
		let left_pressed = ctx.input.button_down(input::keys::KeyA);

		let yaw_left_pressed = ctx.input.button_down(input::keys::ArrowLeft) || ctx.input.button_down(input::keys::Numpad4);
		let yaw_right_pressed = ctx.input.button_down(input::keys::ArrowRight) || ctx.input.button_down(input::keys::Numpad6);

		let pitch_up_pressed = ctx.input.button_down(input::keys::ArrowUp) || ctx.input.button_down(input::keys::Numpad8);
		let pitch_down_pressed = ctx.input.button_down(input::keys::ArrowDown) || ctx.input.button_down(input::keys::Numpad2);

		let yaw_key_delta = match (yaw_right_pressed, yaw_left_pressed) {
			(true, false) => 1.0,
			(false, true) => -1.0,
			_ => 0.0
		};

		let pitch_key_delta = match (pitch_up_pressed, pitch_down_pressed) {
			(true, false) => 1.0,
			(false, true) => -1.0,
			_ => 0.0
		};

		{
			let (dyaw, dpitch) = ctx.input.mouse_delta_radians().map_or((0.0, 0.0), Vec2::to_tuple);

			// https://github.com/id-Software/Quake-III-Arena/blob/dbe4ddb10315479fc00086f08e25d968b4b43c49/code/client/cl_input.c#L293
			// https://github.com/id-Software/Quake-III-Arena/blob/dbe4ddb10315479fc00086f08e25d968b4b43c49/code/client/cl_main.c#L2300
			let pitch_yaw_speed = 140.0f32.to_radians();
			let dt = 1.0/60.0;

			let key_yaw = yaw_key_delta * pitch_yaw_speed * dt;
			let key_pitch = pitch_key_delta * pitch_yaw_speed * dt;

			self.placement.yaw += dyaw + key_yaw;
			self.placement.yaw %= TAU;

			let pitch_limit = PI/2.0;
			self.pitch = (self.pitch - dpitch - key_pitch).clamp(-pitch_limit, pitch_limit);
		}



		let base_speed = 1.0/60.0;
		let speed = match (ctx.input.button_down(input::keys::Shift), ctx.input.button_down(input::keys::Alt)) {
			(true, false) => 2.0 * base_speed,
			(false, true) => 0.25 * base_speed,
			_ => base_speed,
		};

		if self.free_cam {
			// TODO(pat.m): figure out why these need to be negated :(
			// yaw at least I think is because I'm using Vec2::to_x0y, but pitch??
			let yaw_orientation = Quat::from_yaw(-self.placement.yaw);
			let orientation = yaw_orientation * Quat::from_pitch(-self.pitch);

			let right = yaw_orientation.right();
			let forward = orientation.forward();

			if forward_pressed {
				self.free_pos += forward * speed;
			}

			if back_pressed {
				self.free_pos -= forward * speed;
			}

			if right_pressed {
				self.free_pos += right * speed;
			}

			if left_pressed {
				self.free_pos -= right * speed;
			}

		} else {
			let forward = self.placement.forward();
			let right = self.placement.right();

			let mut delta = Vec2::zero();

			if forward_pressed {
				delta += forward * speed;
			}

			if back_pressed {
				delta -= forward * speed;
			}

			if right_pressed {
				delta += right * speed;
			}

			if left_pressed {
				delta -= right * speed;
			}

			self.try_move_by(world, processed_world, delta);
		}
	}
}


// TODO(pat.m): some kind of transform/connectivity cache

impl Player {
	fn try_move_by(&mut self, world: &World, processed_world: &ProcessedWorld, delta: Vec2) {
		if delta.dot(delta) <= 0.00001 {
			return;
		}

		// TODO(pat.m): limit movement by delta length to avoid teleporting

		let current_room = &world.rooms[self.placement.room_index];
		let mut desired_position = self.placement.position + delta;

		fn collide_vertex(desired_position: &mut Vec2, vertex: Vec2, radius: f32) {
			let desired_delta = *desired_position - vertex;
			let penetration = radius - desired_delta.length();
			// TODO(pat.m): this should involve the incoming direction so that a large
			// enough delta can't just pass through the vertex

			if penetration > 0.0 {
				let direction = desired_delta.normalize();
				*desired_position += direction * penetration;
			}
		}

		// Collide with room verts
		for vertex in current_room.wall_vertices.iter() {
			collide_vertex(&mut desired_position, *vertex, PLAYER_RADIUS);
		}

		// Collide with walls
		for wall_index in 0..current_room.walls.len() {
			let (wall_start, wall_end) = current_room.wall_vertices(wall_index);

			let wall_direction = (wall_end - wall_start).normalize();
			let wall_length = (wall_end - wall_start).length();

			let desired_delta_wall_space = desired_position - wall_start;
			let wall_penetration = wall_direction.wedge(desired_delta_wall_space);

			// ASSUME: rooms are convex, and walls are specified in CCW order.

			// Clockwise wedge product means desired position is on the 'inside'
			if wall_penetration + PLAYER_RADIUS < 0.0 {
				continue
			}

			// If the wall ends a long way away then don't continue
			let distance_along_wall = wall_direction.dot(desired_delta_wall_space);
			if distance_along_wall < 0.0 || distance_along_wall >= wall_length {
				continue
			}

			// We have some kind of intersection here - figure out if we need to transition to another room
			// or if we need to slide against the wall
			let wall_id = WallId{room_index: self.placement.room_index, wall_index};
			if let Some(connection_info) = processed_world.connection_for(wall_id) {
				// Collide with the virtual aperture verts
				collide_vertex(&mut desired_position, connection_info.aperture_start, PLAYER_RADIUS);
				collide_vertex(&mut desired_position, connection_info.aperture_end, PLAYER_RADIUS);

				let aperture_center = wall_length/2.0 + connection_info.aperture_offset;
				let intersection_dist_from_center = (aperture_center - distance_along_wall).abs();

				// Target room must be tall enough and the step must not be too steep
				let can_transition_to_opposing_room = PLAYER_HEIGHT < connection_info.aperture_height
					&& connection_info.height_difference.abs() < PLAYER_HEIGHT * PLAYER_MAX_STEP_HEIGHT_PERCENTAGE;

				// If we're transitioning through the aperture then we need to transition to the opposing room.
				// Otherwise just slide as normal.
				if can_transition_to_opposing_room && intersection_dist_from_center < connection_info.aperture_extent {
					if wall_penetration < 0.0 {
						continue
					}

					self.placement.room_index = connection_info.target_id.room_index;
					self.placement.position = connection_info.source_to_target * desired_position;

					// Apply yaw offset
					self.placement.yaw += connection_info.yaw_delta;

					// TODO(pat.m): figure out another way to do this
					self.hack_height_change = Some(connection_info.height_difference);

					// TODO(pat.m): collide with walls in opposing wall as well
					return;
				}
			}

			// Slide along wall
			desired_position -= wall_direction.perp() * (wall_penetration + PLAYER_RADIUS);
		}

		// If we get here, no transitions have happened and desired_position has been adjusted to remove wall collisions
		self.placement.position = desired_position;
	}
}