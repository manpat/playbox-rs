use crate::prelude::*;
use model::{Placement, WallId, World, ProcessedWorld};

/// Ratio of player height to max step distance.
pub const PLAYER_MAX_STEP_HEIGHT_PERCENTAGE: f32 = 0.5;
pub const PLAYER_RADIUS: f32 = 0.1;

// TODO(pat.m): maybe this should just be 'camera'? only some of this should be serialized
#[derive(Debug, Clone)]
pub struct Player {
	pub placement: Placement,
	pub pitch: f32,

	pub height: f32,

	// TODO(pat.m): these should be separate to player
	pub free_pos: Vec3,
	pub free_cam: bool,

	// TODO(pat.m): yucky :((((
	pub hack_height_change: Option<f32>,
}

impl Player {
	pub fn handle_input(&mut self, ctx: &mut Context<'_>, world: &World, processed_world: &ProcessedWorld) {
		self.hack_height_change = None;

		if ctx.input.button_just_down(input::keys::KeyV) {
			self.free_cam = !self.free_cam;

			if !self.free_cam {
				self.free_pos = Vec3::zero();
			}
		}

		{
			let (dx, dy) = ctx.input.mouse_delta().map_or((0.0, 0.0), Vec2::to_tuple);
			self.placement.yaw += dx * TAU;
			self.placement.yaw %= TAU;

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
			let yaw_orientation = Quat::from_yaw(-self.placement.yaw);
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
			let right = Vec2::from_angle(self.placement.yaw);
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
				let wall_center = wall_length/2.0 + connection_info.aperture_offset;
				let aperture_a = wall_start + (wall_center - connection_info.aperture_extent) * wall_direction;
				let aperture_b = wall_start + (wall_center + connection_info.aperture_extent) * wall_direction;
				let intersection_dist_from_center = (wall_center - distance_along_wall).abs();

				// Collide with the virtual aperture verts
				collide_vertex(&mut desired_position, aperture_a, PLAYER_RADIUS);
				collide_vertex(&mut desired_position, aperture_b, PLAYER_RADIUS);

				// Target room must be tall enough and the step must not be too steep
				let can_transition_to_opposing_room = self.height < connection_info.aperture_height
					&& connection_info.height_difference.abs() < self.height * PLAYER_MAX_STEP_HEIGHT_PERCENTAGE;

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