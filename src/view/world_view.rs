use crate::prelude::*;
use model::*;

mod room_mesh_builder;
use room_mesh_builder::*;


pub struct WorldView {
	room_mesh_infos: Vec<RoomMeshInfo>,
	vbo: gfx::BufferName,
	ebo: gfx::BufferName,

	v_shader: gfx::ShaderHandle,

	// Every visible instance of each room
	visible_rooms: Vec<RoomInstance>,

	message_bus: MessageBus,
	change_subscription: Subscription<WorldChangedEvent>,
}

impl WorldView {
	pub fn new(gfx: &mut gfx::System, world: &World, processed_world: &ProcessedWorld, message_bus: MessageBus) -> anyhow::Result<Self> {
		let mut room_builder = RoomMeshBuilder::new(world, processed_world);
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
			visible_rooms: Vec::new(),

			change_subscription: message_bus.subscribe(),
			message_bus,
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System, world: &World, processed_world: &ProcessedWorld, viewer_placement: Placement) {
		// Draw room you're in
		// then for each wall,
		// 	check if it has a neighbouring room, and if so
		// 	calculate transform between connected walls, and build that room,
		// 	using wall intersection to calculate a frustum to cull by

		if self.message_bus.any(&self.change_subscription) {
			let mut room_builder = RoomMeshBuilder::new(world, processed_world);

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


		self.build_visibility_graph(world, processed_world, viewer_placement);

		// Draw
		let mut group = gfx.frame_encoder.command_group(gfx::FrameStage::Main);

		for &RoomInstance{room_index, room_to_world, clip_by, height_offset} in self.visible_rooms.iter() {
			let room_info = &self.room_mesh_infos[room_index];
			let index_size = std::mem::size_of::<u32>() as u32;

			let [x,z,w] = room_to_world.columns();
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
				Some(ClipState{left_aperture, right_aperture, local_viewer_position, aperture_plane, ..}) => {
					let pos_to_left = left_aperture - local_viewer_position;
					let pos_to_right = right_aperture - local_viewer_position;

					let normal_a = pos_to_right.perp().normalize();
					let dist_a = local_viewer_position.dot(normal_a);

					let normal_b = -pos_to_left.perp().normalize();
					let dist_b = local_viewer_position.dot(normal_b);

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
	}

	fn build_visibility_graph(&mut self, world: &World, processed_world: &ProcessedWorld, viewer_placement: Placement) {
		const MAX_DEPTH: i32 = 50;

		self.visible_rooms.clear();
		self.visible_rooms.push(RoomInstance {
			room_index: viewer_placement.room_index,
			room_to_world: Mat2x3::identity(),
			height_offset: 0.0,
			clip_by: None,
		});

		// Viewer forward vector
		let viewer_forward = Vec2::from_angle(viewer_placement.yaw - PI / 2.0);

		let mut instance_index = 0;

		// Build visibility graph
		while let Some(&RoomInstance{room_index, room_to_world, clip_by, height_offset}) = self.visible_rooms.get(instance_index) {
			instance_index += 1;

			let depth = clip_by.map_or(0, |c| c.depth);
			if depth >= MAX_DEPTH {
				continue
			}

			let num_walls = world.rooms[room_index].walls.len();
			let local_viewer_position = clip_by.map_or(viewer_placement.position, |c| c.local_viewer_position);

			for wall_index in 0..num_walls {
				let wall_id = WallId{room_index, wall_index};

				if let Some(connection_info) = processed_world.connection_for(wall_id) {
					let (left_aperture, right_aperture, unclipped_left_aperture) = {
						let start_vertex = connection_info.aperture_start;
						let end_vertex = connection_info.aperture_end;

						// If the aperture we're considering isn't CCW from our position then cull it and the room it connects to.
						if (end_vertex - local_viewer_position).wedge(start_vertex - local_viewer_position) < 0.0 {
							continue;
						}

						// This fudge factor is to deal with the fact that we can look up and see behind us.
						const BEHIND_VIEWER_BUFFER_DIST: f32 = 10.0;

						let start_vertex_invisible = viewer_forward.dot(room_to_world * start_vertex) < -BEHIND_VIEWER_BUFFER_DIST;
						let end_vertex_invisible = viewer_forward.dot(room_to_world * end_vertex) < -BEHIND_VIEWER_BUFFER_DIST;

						// If the aperture is completely behind us then cull it and the room it connects to.
						if start_vertex_invisible && end_vertex_invisible {
							continue;
						}


						if let Some(clip_state) = &clip_by {
							match clip_wall_segment((start_vertex, end_vertex), clip_state) {
								Some((left, right)) => (left, right, start_vertex),
								None => continue,
							}

						} else {
							(start_vertex, end_vertex, start_vertex)
						}
					};


					let total_transform = room_to_world * connection_info.target_to_source;

					// TODO(pat.m): this is kind of a mess, and wouldn't really be necessary if clip_wall_segment actually clipped things.
					// but it works
					let aperture_normal = connection_info.source_to_target * connection_info.wall_normal.extend(0.0);
					let aperture_plane = aperture_normal.extend(aperture_normal.dot(connection_info.source_to_target * unclipped_left_aperture));

					self.visible_rooms.push(RoomInstance {
						room_index: connection_info.target_id.room_index,
						room_to_world: total_transform,
						height_offset: height_offset + connection_info.height_difference,

						clip_by: Some(ClipState {
							depth: depth+1,

							// All of these should be in the space of the target room
							local_viewer_position: connection_info.source_to_target * local_viewer_position,
							left_aperture: connection_info.source_to_target * left_aperture,
							right_aperture: connection_info.source_to_target * right_aperture,

							aperture_plane,
						})
					});
				}
			}
		}
	}
}






#[derive(Debug, Copy, Clone, Default)]
struct ClipState {
	depth: i32,

	local_viewer_position: Vec2,
	left_aperture: Vec2,
	right_aperture: Vec2,
	aperture_plane: Vec3,
}

fn clip_wall_segment((mut left_vertex, mut right_vertex): (Vec2, Vec2), clip_by: &ClipState) -> Option<(Vec2, Vec2)> {
	let &ClipState{left_aperture, right_aperture, local_viewer_position, ..} = clip_by;

	let pos_to_left_clip = left_aperture - local_viewer_position;
	let pos_to_right_clip = right_aperture - local_viewer_position;

	let pos_to_left_vert = left_vertex - local_viewer_position;
	let pos_to_right_vert = right_vertex - local_viewer_position;

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


struct RoomInstance {
	room_index: usize,
	room_to_world: Mat2x3,
	height_offset: f32,
	clip_by: Option<ClipState>,
}