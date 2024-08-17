use crate::prelude::*;
use world::*;


pub struct WorldView {
	room_mesh_infos: Vec<RoomMeshInfo>,
	vbo: gfx::BufferName,
	ebo: gfx::BufferName,

	v_shader: gfx::ShaderHandle,

	message_bus: MessageBus,
	change_subscription: Subscription<WorldChangedEvent>,
}

impl WorldView {
	pub fn new(gfx: &mut gfx::System, world: &World, message_bus: MessageBus) -> anyhow::Result<Self> {
		let mut room_builder = RoomMeshBuilder {
			world,
			vertices: Vec::new(),
			indices: Vec::new(),

			ceiling_height: 1.0,
		};

		let mut room_mesh_infos = Vec::new();

		for room_idx in 0..world.rooms.len() {
			let info = room_builder.build_room(room_idx);
			room_mesh_infos.push(info);
		}

		let vbo = gfx.core.create_buffer();
		let ebo = gfx.core.create_buffer();

		gfx.core.upload_immutable_buffer_immediate(vbo, &room_builder.vertices);
		gfx.core.upload_immutable_buffer_immediate(ebo, &room_builder.indices);

		Ok(Self {
			room_mesh_infos,
			vbo, ebo,

			v_shader: gfx.resource_manager.request(gfx::LoadShaderRequest::from("shaders/standard-room.vs.glsl")?),

			change_subscription: message_bus.subscribe(),
			message_bus,
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System, sprites: &mut super::Sprites, world: &World, world_position: WorldPosition) {
		// Draw room you're in
		// then for each wall,
		// 	check if it has a neighbouring room, and if so
		// 	calculate transform between connected walls, and build that room,
		// 	using wall intersection to calculate a frustum to cull by
		if !self.message_bus.poll(&self.change_subscription).is_empty() {
			let mut room_builder = RoomMeshBuilder {
				world,
				vertices: Vec::new(),
				indices: Vec::new(),

				ceiling_height: 1.0,
			};

			self.room_mesh_infos.clear();

			for room_idx in 0..world.rooms.len() {
				let info = room_builder.build_room(room_idx);
				self.room_mesh_infos.push(info);
			}

			gfx.core.destroy_buffer(self.vbo);
			gfx.core.destroy_buffer(self.ebo);

			self.vbo = gfx.core.create_buffer();
			self.ebo = gfx.core.create_buffer();

			gfx.core.upload_immutable_buffer_immediate(self.vbo, &room_builder.vertices);
			gfx.core.upload_immutable_buffer_immediate(self.ebo, &room_builder.indices);
		}

		let initial_transform = Mat2x3::rotate_translate(0.0, -world_position.local_position);

		const MAX_DEPTH: i32 = 50;

		struct Entry {
			room_index: usize,
			transform: Mat2x3,
			clip_by: Option<ClipState>,
		}

		let mut room_stack = vec![
			Entry {
				room_index: world_position.room_index,
				transform: initial_transform,
				clip_by: None,
			}
		];

		let mut group = gfx.frame_encoder.command_group(gfx::FrameStage::Main);
		let rm = &mut gfx.resource_manager;

		while let Some(Entry{room_index, transform, clip_by}) = room_stack.pop() {
			// Draw
			{
				let room_info = &self.room_mesh_infos[room_index];
				let index_size = std::mem::size_of::<u32>();

				let [x,z,w] = transform.columns();
				let transform = Mat3x4::from_columns([
					x.to_x0y(),
					Vec3::from_y(1.0),
					z.to_x0y(),
					w.to_x0y()
				]);

				// sprites.billboard(w.to_x0y() + Vec3::from_y(0.3), Vec2::splat(0.1), Color::white());

				#[derive(Copy, Clone)]
				#[repr(C)]
				struct RoomUniforms {
					transform: Mat3x4,
					plane_0: Vec4,
					plane_1: Vec4,
				}

				let (plane_0, plane_1) = match clip_by {
					Some(ClipState{left_apperture, right_apperture, local_position, ..}) => {
						let pos_to_left = left_apperture - local_position;
						let pos_to_right = right_apperture - local_position;

						let normal_a = pos_to_right.perp().normalize();
						let dist_a = local_position.dot(normal_a);

						let normal_b = -pos_to_left.perp().normalize();
						let dist_b = local_position.dot(normal_b);

						(normal_a.to_x0y().extend(dist_a), normal_b.to_x0y().extend(dist_b))
					}

					None => (Vec4::from_w(-1.0), Vec4::from_w(-1.0)),
				};

				group.draw(self.v_shader, rm.flat_fs_shader)
					.elements(room_info.num_elements)
					.ssbo(0, self.vbo)
					.ubo(1, &[RoomUniforms {
						transform,
						plane_0,
						plane_1,
					}])
					.indexed(gfx::bindings::BufferBindSource::Name {
						name: self.ebo,
						range: Some(gfx::BufferRange {
							offset: room_info.base_index as usize * index_size,
							size: room_info.num_elements as usize * index_size,
						})
					});
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

			fn try_add_connection(room_stack: &mut Vec<Entry>, world: &World, current_wall_id: GlobalWallId, target_wall_id: GlobalWallId,
				transform: &Mat2x3, clip_by: &Option<ClipState>, local_position: Vec2, depth: i32)
			{
				let local_position = clip_by.map_or(local_position, |c| c.local_position);

				let (left_apperture, right_apperture) = {
					let (start_vertex, end_vertex) = world.wall_vertices(current_wall_id);

					// If the apperture we're considering isn't CCW from our perspective then cull it and the room it connects to.
					if (end_vertex - local_position).wedge(start_vertex - local_position) < 0.0 {
						return;
					}

					let wall_length = (end_vertex - start_vertex).length();
					let wall_dir = (end_vertex - start_vertex) / wall_length;
					let opposing_wall_length = {
						let (wall_start, wall_end) = world.wall_vertices(target_wall_id);
						(wall_end - wall_start).length()
					};

					let apperture_half_size = wall_length.min(opposing_wall_length) / 2.0;
					let left_vertex = start_vertex + wall_dir * (wall_length/2.0 - apperture_half_size);
					let right_vertex = start_vertex + wall_dir * (wall_length/2.0 + apperture_half_size);

					if let Some(clip_state) = &clip_by {
						match clip_wall_segment((left_vertex, right_vertex), clip_state) {
							Some(wall) => wall,
							None => return,
						}

					} else {
						(left_vertex, right_vertex)
					}

				};

				let portal_transform = calculate_portal_transform(world, current_wall_id, target_wall_id);
				let inv_portal_transform = portal_transform.inverse();
				let total_transform = *transform * portal_transform;

				room_stack.push(Entry {
					room_index: target_wall_id.room_index,
					transform: total_transform,
					clip_by: Some(ClipState {
						depth: depth+1,

						// All of these should be in the space of the target room
						local_position: inv_portal_transform * local_position,
						left_apperture: inv_portal_transform * left_apperture,
						right_apperture: inv_portal_transform * right_apperture,
					})
				});
			}

			for (current_wall_id, target_wall_id) in connections {
				try_add_connection(&mut room_stack, world, current_wall_id, target_wall_id, &transform, &clip_by, world_position.local_position, depth);

				// If we connect to the same room then we need to draw again with the inverse transform to make sure both walls get recursed through
				if current_wall_id.room_index == target_wall_id.room_index {
					try_add_connection(&mut room_stack, world, target_wall_id, current_wall_id, &transform, &clip_by, world_position.local_position, depth);
				}
			}
		}

	}
}

struct RoomMeshInfo {
	base_vertex: u32,
	base_index: u32,
	num_elements: u32,
}

struct RoomMeshBuilder<'a> {
	world: &'a World,
	vertices: Vec<gfx::StandardVertex>,
	indices: Vec<u32>,

	ceiling_height: f32,
}

impl RoomMeshBuilder<'_> {
	fn add_convex<VS>(&mut self, vs: VS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
	{
		let vs = vs.into_iter();
		let start_index = self.vertices.len() as u32;
		let indices = (1..vs.len() as u32 - 1)
			.flat_map(|i| [start_index, start_index + i, start_index + i + 1]);

		let color = color.into();
		let vertices = vs.map(|pos| gfx::StandardVertex::new(pos, Vec2::zero(), color));

		self.vertices.extend(vertices);
		self.indices.extend(indices);
	}

	fn build_room(&mut self, room_index: usize) -> RoomMeshInfo {
		let base_vertex = self.vertices.len() as u32;
		let base_index = self.indices.len() as u32;

		let room = &self.world.rooms[room_index];
		let up = Vec3::from_y(self.ceiling_height);

		let floor_verts = room.wall_vertices.iter().map(|&v| v.to_x0y());
		let ceiling_verts = floor_verts.clone().rev().map(|v| v + up);

		// Floor/Ceiling
		self.add_convex(floor_verts, room.floor_color);
		self.add_convex(ceiling_verts, room.ceiling_color);

		// Walls
		for wall_index in 0..room.walls.len() {
			let wall_id = GlobalWallId{room_index, wall_index};
			let connection = self.world.connections.iter()
				.find_map(|&(a, b)|
					if a == wall_id {
						Some(b)
					} else if b == wall_id {
						Some(a)
					} else {
						None
					}
				);

			self.build_wall(wall_id, connection);
		}

		let num_elements = self.indices.len() as u32 - base_index;

		RoomMeshInfo {base_vertex, base_index, num_elements}
	}

	fn build_wall(&mut self, GlobalWallId{room_index, wall_index}: GlobalWallId, opposing_wall_id: Option<GlobalWallId>) {
		let room = &self.world.rooms[room_index];

		let wall_color = room.walls[wall_index].color;
		let (start_vertex, end_vertex) = room.wall_vertices(wall_index);

		let start_vertex_3d = start_vertex.to_x0y();
		let end_vertex_3d = end_vertex.to_x0y();

		let up = Vec3::from_y(self.ceiling_height);

		if let Some(opposing_wall_id) = opposing_wall_id {
			// Connected walls may be different lengths, so we need to calculate the aperture that we can actually
			// pass through.
			let opposing_wall_length = {
				let opposing_room = &self.world.rooms[opposing_wall_id.room_index];
				let (wall_start, wall_end) = opposing_room.wall_vertices(opposing_wall_id.wall_index);
				(wall_end - wall_start).length()
			};

			let wall_length = (end_vertex - start_vertex).length();
			let wall_dir = (end_vertex - start_vertex) / wall_length;

			let apperture_half_size = wall_length.min(opposing_wall_length) / 2.0;
			let left_vertex = start_vertex + wall_dir * (wall_length/2.0 - apperture_half_size);
			let right_vertex = start_vertex + wall_dir * (wall_length/2.0 + apperture_half_size);

			let left_vertex_3d = left_vertex.to_x0y();
			let right_vertex_3d = right_vertex.to_x0y();

			let verts = [
				start_vertex_3d,
				start_vertex_3d + up,
				left_vertex_3d + up,
				left_vertex_3d,
			];

			self.add_convex(verts, wall_color);

			let verts = [
				right_vertex_3d,
				right_vertex_3d + up,
				end_vertex_3d + up,
				end_vertex_3d,
			];

			self.add_convex(verts, wall_color);

		} else {
			let verts = [
				start_vertex_3d,
				start_vertex_3d + up,
				end_vertex_3d + up,
				end_vertex_3d,
			];

			self.add_convex(verts, wall_color);
		}
	}
}






#[derive(Debug, Copy, Clone, Default)]
struct ClipState {
	depth: i32,

	local_position: Vec2,
	left_apperture: Vec2,
	right_apperture: Vec2,
}

fn clip_wall_segment((mut left_vertex, mut right_vertex): (Vec2, Vec2), clip_by: &ClipState) -> Option<(Vec2, Vec2)> {
	let &ClipState{left_apperture, right_apperture, local_position, ..} = clip_by;

	let pos_to_left_clip = left_apperture - local_position;
	let pos_to_right_clip = right_apperture - local_position;
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
	if pos_to_left_vert.wedge(pos_to_left_clip) < 0.0 {
		left_vertex = left_apperture;
	}

	if pos_to_right_vert.wedge(pos_to_right_clip) > 0.0 {
		right_vertex = right_apperture;
	}

	Some((left_vertex, right_vertex))
}