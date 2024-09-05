use crate::prelude::*;
use model::*;

mod room_mesh_builder;
use room_mesh_builder::*;


pub struct WorldView {
	room_mesh_infos: Vec<RoomMeshInfo>,
	vbo: gfx::BufferName,
	ebo: gfx::BufferName,

	v_shader: gfx::ShaderHandle,

	message_bus: MessageBus,
	change_subscription: Subscription<WorldChangedEvent>,

	height_offset: f32,
}

impl WorldView {
	pub fn new(gfx: &mut gfx::System, world: &World, message_bus: MessageBus) -> anyhow::Result<Self> {
		let mut room_builder = RoomMeshBuilder::new(world);
		let mut room_mesh_infos = Vec::new();

		for room_idx in 0..world.rooms.len() {
			let info = room_builder.build_room(room_idx);
			room_mesh_infos.push(info);
		}

		let vbo = gfx.core.create_buffer();
		let ebo = gfx.core.create_buffer();

		room_builder.upload(gfx, vbo, ebo);

		Ok(Self {
			room_mesh_infos,
			vbo, ebo,

			v_shader: gfx.resource_manager.request(gfx::LoadShaderRequest::from("shaders/standard-room.vs.glsl")?),

			change_subscription: message_bus.subscribe(),
			message_bus,

			height_offset: 0.0,
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System, _sprites: &mut super::Sprites, world: &World, viewer_placement: Placement, height_change: Option<f32>) {
		// Draw room you're in
		// then for each wall,
		// 	check if it has a neighbouring room, and if so
		// 	calculate transform between connected walls, and build that room,
		// 	using wall intersection to calculate a frustum to cull by
		if !self.message_bus.poll(&self.change_subscription).is_empty() {
			let mut room_builder = RoomMeshBuilder::new(world);

			self.room_mesh_infos.clear();

			for room_idx in 0..world.rooms.len() {
				let info = room_builder.build_room(room_idx);
				self.room_mesh_infos.push(info);
			}

			gfx.core.destroy_buffer(self.vbo);
			gfx.core.destroy_buffer(self.ebo);

			self.vbo = gfx.core.create_buffer();
			self.ebo = gfx.core.create_buffer();

			room_builder.upload(gfx, self.vbo, self.ebo);
		}

		// TODO(pat.m): find another way to do this
		if let Some(height_change) = height_change {
			self.height_offset += height_change;
		}

		if self.height_offset.abs() > 0.02 {
			self.height_offset -= self.height_offset.signum() * 0.02;

		} else {
			self.height_offset = 0.0;
		}


		let initial_transform = Mat2x3::rotate_translate(0.0, -viewer_placement.position);

		const MAX_DEPTH: i32 = 50;

		struct Entry {
			room_index: usize,
			transform: Mat2x3,
			height_offset: f32,
			clip_by: Option<ClipState>,
		}

		let mut room_stack = vec![
			Entry {
				room_index: viewer_placement.room_index,
				transform: initial_transform,
				height_offset: self.height_offset,
				clip_by: None,
			}
		];

		let mut group = gfx.frame_encoder.command_group(gfx::FrameStage::Main);

		while let Some(Entry{room_index, transform, clip_by, height_offset}) = room_stack.pop() {
			// Draw
			{
				let room_info = &self.room_mesh_infos[room_index];
				let index_size = std::mem::size_of::<u32>() as u32;

				let [x,z,w] = transform.columns();
				let transform = Mat3x4::from_columns([
					x.to_x0y(),
					Vec3::from_y(1.0),
					z.to_x0y(),
					w.to_x0y() + Vec3::from_y(height_offset)
				]);

				#[derive(Copy, Clone)]
				#[repr(C)]
				struct RoomUniforms {
					transform: Mat3x4,
					plane_0: Vec4,
					plane_1: Vec4,
					plane_2: Vec4,
				}

				let (plane_0, plane_1, plane_2) = match clip_by {
					Some(ClipState{left_aperture, right_aperture, local_position, aperture_plane, ..}) => {
						let pos_to_left = left_aperture - local_position;
						let pos_to_right = right_aperture - local_position;

						let normal_a = pos_to_right.perp().normalize();
						let dist_a = local_position.dot(normal_a);

						let normal_b = -pos_to_left.perp().normalize();
						let dist_b = local_position.dot(normal_b);

						let plane_0 = normal_a.to_x0y().extend(dist_a);
						let plane_1 = normal_b.to_x0y().extend(dist_b);

						let [x, y, w] = aperture_plane.into();
						let plane_2 = Vec4::new(x, 0.0, y, w);

						(plane_0, plane_1, plane_2)
					}

					None => (Vec4::from_w(-1.0), Vec4::from_w(-1.0), Vec4::from_w(-1.0)),
				};

				group.draw(self.v_shader, gfx::CommonShader::FlatTexturedFragment)
					.elements(room_info.num_elements)
					.ssbo(0, self.vbo)
					.ubo(1, &[RoomUniforms {
						transform,
						plane_0,
						plane_1,
						plane_2,
					}])
					.indexed(self.ebo.with_offset_size(
						room_info.base_index * index_size,
						room_info.num_elements * index_size
					))
					.base_vertex(room_info.base_vertex);
			}

			let depth = clip_by.map_or(0, |c| c.depth);
			if depth >= MAX_DEPTH {
				continue
			}

			let connections = world.connections.iter()
				.filter_map(|&(left, right)| {
					if left.room_index == room_index {
						Some((left, right))
					} else if right.room_index == room_index {
						Some((right, left))
					} else {
						None
					}
				});

			fn try_add_connection(room_stack: &mut Vec<Entry>, world: &World, current_wall_id: WallId, target_wall_id: WallId,
				transform: &Mat2x3, clip_by: &Option<ClipState>, local_position: Vec2, height_offset: f32, depth: i32)
			{
				let local_position = clip_by.map_or(local_position, |c| c.local_position);

				let current_wall = &world.rooms[current_wall_id.room_index].walls[current_wall_id.wall_index];
				let target_wall = &world.rooms[target_wall_id.room_index].walls[target_wall_id.wall_index];

				let (left_aperture, right_aperture, aperture_normal, unclipped_left_aperture) = {
					let (start_vertex, end_vertex) = world.wall_vertices(current_wall_id);

					// If the aperture we're considering isn't CCW from our perspective then cull it and the room it connects to.
					if (end_vertex - local_position).wedge(start_vertex - local_position) < 0.0 {
						return;
					}

					let wall_length = (end_vertex - start_vertex).length();
					let wall_dir = (end_vertex - start_vertex) / wall_length;
					let opposing_wall_length = {
						let (wall_start, wall_end) = world.wall_vertices(target_wall_id);
						(wall_end - wall_start).length()
					};


					let aperture_extent = wall_length.min(opposing_wall_length) / 2.0;
					let aperture_offset = current_wall.horizontal_offset.clamp(aperture_extent-wall_length/2.0, wall_length/2.0-aperture_extent);

					let wall_center = wall_length/2.0 + aperture_offset;

					let left_vertex = start_vertex + wall_dir * (wall_center - aperture_extent);
					let right_vertex = start_vertex + wall_dir * (wall_center + aperture_extent);

					let normal = (right_vertex - left_vertex).normalize().perp();

					if let Some(clip_state) = &clip_by {
						match clip_wall_segment((left_vertex, right_vertex), clip_state) {
							Some((left, right)) => (left, right, normal, left_vertex),
							None => return,
						}

					} else {
						(left_vertex, right_vertex, normal, left_vertex)
					}
				};

				let portal_transform = calculate_portal_transform(world, current_wall_id, target_wall_id);
				let inv_portal_transform = portal_transform.inverse();
				let total_transform = *transform * portal_transform;

				let left_aperture = inv_portal_transform * left_aperture;
				let right_aperture = inv_portal_transform * right_aperture;

				// TODO(pat.m): this is kind of a mess, and wouldn't really be necessary if clip_wall_segment actually clipped things.
				// but it works
				let aperture_normal = inv_portal_transform * aperture_normal.extend(0.0);
				let aperture_plane = aperture_normal.extend(aperture_normal.dot(inv_portal_transform * unclipped_left_aperture));

				let height_difference = current_wall.vertical_offset - target_wall.vertical_offset;

				room_stack.push(Entry {
					room_index: target_wall_id.room_index,
					transform: total_transform,
					height_offset: height_offset + height_difference,

					clip_by: Some(ClipState {
						depth: depth+1,

						// All of these should be in the space of the target room
						local_position: inv_portal_transform * local_position,
						left_aperture,
						right_aperture,

						aperture_plane,
					})
				});
			}

			for (current_wall_id, target_wall_id) in connections {
				try_add_connection(&mut room_stack, world, current_wall_id, target_wall_id, &transform, &clip_by, viewer_placement.position, height_offset, depth);

				// If we connect to the same room then we need to draw again with the inverse transform to make sure both walls get recursed through
				if current_wall_id.room_index == target_wall_id.room_index {
					try_add_connection(&mut room_stack, world, target_wall_id, current_wall_id, &transform, &clip_by, viewer_placement.position, height_offset, depth);
				}
			}
		}

	}
}






#[derive(Debug, Copy, Clone, Default)]
struct ClipState {
	depth: i32,

	local_position: Vec2,
	left_aperture: Vec2,
	right_aperture: Vec2,
	aperture_plane: Vec3,
}

fn clip_wall_segment((mut left_vertex, mut right_vertex): (Vec2, Vec2), clip_by: &ClipState) -> Option<(Vec2, Vec2)> {
	let &ClipState{left_aperture, right_aperture, local_position, ..} = clip_by;

	let pos_to_left_clip = left_aperture - local_position;
	let pos_to_right_clip = right_aperture - local_position;

	let pos_to_left_vert = left_vertex - local_position;
	let pos_to_right_vert = right_vertex - local_position;

	// Full cull
	if pos_to_right_vert.wedge(pos_to_left_clip) < 0.0 {
		return None
	}

	if pos_to_left_vert.wedge(pos_to_right_clip) > 0.0 {
		return None
	}

	// Clip
	let wedge_product = pos_to_left_vert.wedge(pos_to_left_clip);
	if wedge_product < 0.0 {
		// TODO(pat.m): actually clip here - will help later
		left_vertex = left_aperture;
	}

	let wedge_product = pos_to_right_vert.wedge(pos_to_right_clip);
	if wedge_product > 0.0 {
		// TODO(pat.m): actually clip here - will help later
		right_vertex = right_aperture;
	}

	Some((left_vertex, right_vertex))
}