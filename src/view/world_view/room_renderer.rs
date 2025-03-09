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
	let mut room_builder = RoomMeshBuilder::new(processed_world);

	let mut room_mesh_infos = slotmap::SecondaryMap::with_capacity(processed_world.geometry().rooms.len());

	for room_id in processed_world.geometry().rooms.keys() {
		let info = room_builder.build_room(room_id);
		room_mesh_infos.insert(room_id, info);
	}

	room_builder.upload(gfx, vbo, ebo, light_buffer);

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