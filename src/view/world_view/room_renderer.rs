use crate::prelude::*;
use model::*;

use super::room_mesh_builder::*;


pub struct RoomRenderer {
	room_mesh_infos: slotmap::SecondaryMap<RoomId, RoomMeshInfo>,
	instances: slotmap::SecondaryMap<RoomId, SmallVec<[RoomUniforms; 4]>>,

	vbo: gfx::BufferName,
	ebo: gfx::BufferName,
	light_buffer: gfx::BufferName,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,
	texture: gfx::ImageHandle,
}

impl RoomRenderer {
	pub fn new(gfx: &mut gfx::System, processed_world: &ProcessedWorld) -> anyhow::Result<Self> {
		let vbo = gfx.core.create_buffer();
		let ebo = gfx.core.create_buffer();
		let light_buffer = gfx.core.create_buffer();

		gfx.core.set_debug_label(vbo, "Room vertex buffer");
		gfx.core.set_debug_label(ebo, "Room index buffer");
		gfx.core.set_debug_label(light_buffer, "Room light buffer");

		let room_mesh_infos = build_room_buffers(gfx, processed_world, vbo, ebo, light_buffer);


		Ok(RoomRenderer {
			room_mesh_infos,
			instances: Default::default(),

			vbo, ebo,
			light_buffer,

			v_shader: gfx.resource_manager.load_vertex_shader("shaders/room.vs.glsl"),
			f_shader: gfx.resource_manager.load_fragment_shader("shaders/room.fs.glsl"),
			texture: gfx.resource_manager.load_image_array("World Textures", &[
				"images/dumb-brick.png",
				"images/dumb-brick2.png",
				"images/dumb-tile.png",
				// "images/coolcat.png",
				// "images/coolcat.png",
				// "images/coolcat.png",
				// "images/coolcat.png",
			]),
		})
	}

	pub fn rebuild(&mut self, gfx: &mut gfx::System, processed_world: &ProcessedWorld) {
		gfx.core.destroy_buffer(self.vbo);
		gfx.core.destroy_buffer(self.ebo);
		gfx.core.destroy_buffer(self.light_buffer);

		self.vbo = gfx.core.create_buffer();
		self.ebo = gfx.core.create_buffer();
		self.light_buffer = gfx.core.create_buffer();

		gfx.core.set_debug_label(self.vbo, "Room vertex buffer");
		gfx.core.set_debug_label(self.ebo, "Room index buffer");
		gfx.core.set_debug_label(self.light_buffer, "Room light buffer");

		self.room_mesh_infos = build_room_buffers(gfx, processed_world, self.vbo, self.ebo, self.light_buffer);
	}

	pub fn add_instance(&mut self, room_id: RoomId, transform: Mat3x4, planes: &[Vec4; 3]) {
		let instance_list = self.instances.entry(room_id)
			.unwrap()
			.or_default();

		instance_list.push(RoomUniforms {
			transform,
			planes: planes.clone()
		});
	}

	pub fn reset(&mut self) {
		self.instances.clear();
	}

	pub fn draw(&self, encoder: &mut gfx::FrameEncoder) {
		let index_size = std::mem::size_of::<u32>() as u32;

		let mut group = encoder.command_group(gfx::FrameStage::Main);

		for (room_id, instance_list) in self.instances.iter() {
			let mesh_info = &self.room_mesh_infos[room_id];

			let instance_data_upload = group.upload(instance_list);

			group.draw(self.v_shader, self.f_shader)
				.elements(mesh_info.num_elements)
				.instances(instance_list.len() as u32)
				.indexed(self.ebo.with_offset_size(
					mesh_info.base_index * index_size,
					mesh_info.num_elements * index_size
				))
				.base_vertex(mesh_info.base_vertex)
				.sampled_image(0, self.texture, gfx::CommonSampler::NearestRepeat)
				.ssbo(0, self.vbo)
				.ssbo(1, instance_data_upload)
				.ssbo(2, &[mesh_info.base_light, mesh_info.num_lights])
				.ssbo(3, self.light_buffer);
		}
	}
}




#[derive(Copy, Clone)]
#[repr(C)]
struct RoomUniforms {
	transform: Mat3x4,
	planes: [Vec4; 3],
}



fn build_room_buffers(gfx: &mut gfx::System, processed_world: &ProcessedWorld,
	vbo: gfx::BufferName, ebo: gfx::BufferName, light_buffer: gfx::BufferName) -> slotmap::SecondaryMap<RoomId, RoomMeshInfo>
{
	use slotmap::SecondaryMap;

	let mut room_builder = RoomMeshBuilder::new(processed_world);

	let mut room_mesh_infos = SecondaryMap::with_capacity(processed_world.geometry().rooms.len());

	for room_id in processed_world.geometry().rooms.keys() {
		let info = room_builder.build_room(room_id);
		room_mesh_infos.insert(room_id, info);
	}

	struct QueueEntry {
		room_id: RoomId,
		from_wall: WallId,
		transform: Mat3x4,
		depth: u32,

		plane_0: Plane2,
		plane_1: Plane2,
	}

	fn to_transform(target_to_source: Mat2x3, height_difference: f32) -> Mat3x4 {
		let [x,z,w] = target_to_source.columns();
		Mat3x4::from_columns([
			x.to_x0y(),
			Vec3::from_y(1.0),
			z.to_x0y(),
			w.to_x0y() + Vec3::from_y(height_difference)
		])
	}

	let mut room_lights: SecondaryMap<_, Vec<RoomLight>> = SecondaryMap::with_capacity(processed_world.geometry().rooms.len());
	let mut room_queue = SmallVec::<[QueueEntry; 16]>::new();

	// Figure out which rooms touched by each light.
	for object in processed_world.objects.values() {
		let Some(light) = object.as_light() else { continue };

		room_queue.clear();
		room_queue.push(QueueEntry {
			room_id: object.placement.room_id,
			from_wall: WallId::default(), // this should always be invalid.
			transform: Mat3x4::identity(),
			depth: 5,

			plane_0: Plane2::NEGATIVE_INFINITY,
			plane_1: Plane2::NEGATIVE_INFINITY,
		});

		while let Some(room_entry) = room_queue.pop() {
			let local_pos = room_entry.transform * object.placement.position.to_xny(light.height);

			// Push light
			{
				let light_list = room_lights.entry(room_entry.room_id)
					.unwrap().or_default();

				light_list.push(RoomLight {
					local_pos,
					radius: light.radius,
					color: light.color.into(),
					power: light.power,

					plane_0: room_entry.plane_0.to_x0y(),
					plane_1: room_entry.plane_1.to_x0y(),
				});
			}

			log::info!("[build] visited {:?}", room_entry.room_id);

			// Bail if we've hit recursion limit
			let Some(next_depth) = room_entry.depth.checked_sub(1) else { continue };


			// Figure out which walls touch light
			for connection in processed_world.connections_for_room(room_entry.room_id) {
				log::info!("[build] ... check {:?} ({:?} -> {:?})", connection.target_room, connection.source_wall, connection.target_wall);

				if connection.target_wall == room_entry.from_wall {
					// This is the wall we recursed through, skip.
					continue;
				}

				let start_vertex = connection.source_to_target * connection.aperture_start;
				let end_vertex = connection.source_to_target * connection.aperture_end;
				let local_pos2 = local_pos.to_xz();

				// If the aperture we're considering isn't CCW from our position then cull it and the room it connects to.
				if (end_vertex - local_pos2).wedge(start_vertex - local_pos2) < 0.0 {
					continue;
				}

				let plane_0 = Plane2::from_points(start_vertex, local_pos2);
				let plane_1 = Plane2::from_points(local_pos2, end_vertex);

				log::info!("[build] ... recurse");

				room_queue.push(QueueEntry {
					room_id: connection.target_room,
					from_wall: connection.source_wall,
					transform: to_transform(connection.source_to_target, -connection.height_difference) * room_entry.transform,
					depth: next_depth,

					plane_0,
					plane_1,
				});
			}
		}
	}

	// Collect lights for each room into a buffer for upload
	let mut built_light_buffer = Vec::new();
	for (room_id, mesh_info) in room_mesh_infos.iter_mut() {
		let Some(light_list) = room_lights.get(room_id) else { continue };

		mesh_info.base_light = built_light_buffer.len() as u32;
		mesh_info.num_lights = light_list.len() as u32;

		built_light_buffer.extend_from_slice(light_list);
	}

	gfx.core.upload_immutable_buffer_immediate(light_buffer, &built_light_buffer);

	room_builder.upload(gfx, vbo, ebo);

	gfx.core.debug_marker("Uploaded Room Data");

	room_mesh_infos
}





#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct RoomVertex {
	pub pos: Vec3,
	pub uv_packed: [i16; 2],
	pub color_packed: [u16; 4],
	pub texture_index: u32,

	pub _padding: [u32; 1],
}

pub const PIXEL_DENSITY: f32 = 128.0;

impl RoomVertex {
	pub fn new(pos: Vec3, uv: Vec2, color: impl Into<Color>, texture_index: u32) -> RoomVertex {
		let [u, v] = (uv*8.0 * PIXEL_DENSITY).to_vec2i().into();
		let [r, g, b, a] = color.into().to_array();

		RoomVertex {
			pos,
			uv_packed: [u as i16, v as i16],

			color_packed: [
				unorm_to_u16(r),
				unorm_to_u16(g),
				unorm_to_u16(b),
				unorm_to_u16(a),
			],

			texture_index,

			_padding: Default::default(),
		}
	}
}

fn unorm_to_u16(o: f32) -> u16 {
	let umax_f = u16::MAX as f32;
	(o * umax_f).clamp(0.0, umax_f) as u16
}