use crate::prelude::*;
use model::*;


pub struct RoomMeshInfo {
	pub base_vertex: u32,
	pub base_index: u32,
	pub num_elements: u32,
}

pub struct RoomMeshBuilder<'w> {
	world: &'w World,
	processed_world: &'w ProcessedWorld,
	vertices: Vec<RoomVertex>,
	indices: Vec<u32>,
	base_vertex: u32,
	current_texture_index: u32,
}

impl<'w> RoomMeshBuilder<'w> {
	pub fn new(world: &'w World, processed_world: &'w ProcessedWorld) -> Self {
		RoomMeshBuilder {
			world,
			processed_world,
			vertices: Vec::new(),
			indices: Vec::new(),
			base_vertex: 0,
			current_texture_index: 0,
		}
	}

	pub fn set_texture_index(&mut self, texture_index: u32) {
		self.current_texture_index = texture_index;
	}

	pub fn upload(&self, gfx: &gfx::System, vbo: gfx::BufferName, ebo: gfx::BufferName) {
		gfx.core.upload_immutable_buffer_immediate(vbo, &self.vertices);
		gfx.core.upload_immutable_buffer_immediate(ebo, &self.indices);
		gfx.core.debug_marker("Uploaded Room Vertices");
	}
}

impl RoomMeshBuilder<'_> {
	fn add_convex<VS, UVS>(&mut self, vs: VS, uvs: UVS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
			, UVS: IntoIterator<Item=Vec2>
	{
		let vs = vs.into_iter();
		let uvs = uvs.into_iter();
		let start_index = self.vertices.len() as u32 - self.base_vertex;
		let indices = (1..vs.len() as u32 - 1)
			.flat_map(|i| [start_index, start_index + i, start_index + i + 1]);

		let color = color.into();
		let vertices = vs.zip(uvs).map(|(pos, uv)| RoomVertex::new(pos, uv, color, self.current_texture_index));

		self.vertices.extend(vertices);
		self.indices.extend(indices);
	}

	fn add_convex_untextured<VS>(&mut self, vs: VS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
	{
		self.add_convex(vs, std::iter::repeat(Vec2::zero()), color);
	}

	pub fn build_room(&mut self, room_index: usize) -> RoomMeshInfo {
		self.base_vertex = self.vertices.len() as u32;
		let base_index = self.indices.len() as u32;

		let room = &self.world.rooms[room_index];
		let up = Vec3::from_y(room.height);

		// ASSUME: rooms are always convex
		let floor_verts = room.wall_vertices.iter().map(|&v| v.to_x0y());
		let floor_uvs = room.wall_vertices.iter().cloned();
		let ceiling_verts = floor_verts.clone().rev().map(|v| v + up);
		let ceiling_uvs = room.wall_vertices.iter().rev().cloned();

		// Floor/Ceiling
		self.set_texture_index(3);
		self.add_convex(floor_verts, floor_uvs, room.floor_color);
		self.add_convex(ceiling_verts, ceiling_uvs, room.ceiling_color);

		// Walls
		for wall_index in 0..room.walls.len() {
			self.build_wall(WallId{room_index, wall_index});
		}

		// Objects
		self.set_texture_index(0);
		for object in self.world.objects.iter()
			.filter(|o| o.placement.room_index == room_index)
		{
			self.build_object(object);
		}

		let num_elements = self.indices.len() as u32 - base_index;

		RoomMeshInfo {base_vertex: self.base_vertex, base_index, num_elements}
	}

	pub fn build_wall(&mut self, wall_id: WallId) {
		let room = &self.world.rooms[wall_id.room_index];
		let wall = &room.walls[wall_id.wall_index];

		self.set_texture_index(wall_id.wall_index as u32 % 2 + 1);

		let (start_vertex, end_vertex) = room.wall_vertices(wall_id.wall_index);

		let length = (start_vertex - end_vertex).length();
		let start_vertex_3d = start_vertex.to_x0y();
		let end_vertex_3d = end_vertex.to_x0y();

		let up = Vec3::from_y(room.height);

		let Some(connection_info) = self.processed_world.connection_info(wall_id) else {
			let verts = [
				start_vertex_3d,
				start_vertex_3d + up,
				end_vertex_3d + up,
				end_vertex_3d,
			];

			let uvs = [
				Vec2::new(0.0, 0.0),
				Vec2::new(0.0, room.height),
				Vec2::new(length, room.height),
				Vec2::new(length, 0.0),
			];

			self.add_convex(verts, uvs, wall.color);

			return
		};

		let opposing_room = &self.world.rooms[connection_info.target_id.room_index];

		let left_length = (start_vertex - connection_info.aperture_start).length();
		let right_length = (end_vertex - connection_info.aperture_end).length();

		let right_uv_start = length - right_length;

		let left_vertex_3d = connection_info.aperture_start.to_x0y();
		let right_vertex_3d = connection_info.aperture_end.to_x0y();

		// Add left and right room height quads
		let verts = [
			start_vertex_3d,
			start_vertex_3d + up,
			left_vertex_3d + up,
			left_vertex_3d,
		];

		let uvs = [
			Vec2::new(0.0, 0.0),
			Vec2::new(0.0, room.height),
			Vec2::new(left_length, room.height),
			Vec2::new(left_length, 0.0),
		];

		self.add_convex(verts, uvs, wall.color);

		let verts = [
			right_vertex_3d,
			right_vertex_3d + up,
			end_vertex_3d + up,
			end_vertex_3d,
		];

		let uvs = [
			Vec2::new(right_uv_start, 0.0),
			Vec2::new(right_uv_start, room.height),
			Vec2::new(length, room.height),
			Vec2::new(length, 0.0),
		];

		self.add_convex(verts, uvs, wall.color);

		// Add quads above and below the aperture
		if connection_info.height_difference > 0.0 {
			let aperture_bottom = Vec3::from_y(connection_info.height_difference);

			let verts = [
				left_vertex_3d,
				left_vertex_3d + aperture_bottom,
				right_vertex_3d + aperture_bottom,
				right_vertex_3d,
			];

			let uvs = [
				Vec2::new(left_length, 0.0),
				Vec2::new(left_length, aperture_bottom.y),
				Vec2::new(right_uv_start, aperture_bottom.y),
				Vec2::new(right_uv_start, 0.0),
			];

			self.add_convex(verts, uvs, wall.color);
		}

		if connection_info.height_difference + opposing_room.height < room.height {
			let aperture_top = Vec3::from_y(connection_info.height_difference + opposing_room.height);

			let verts = [
				left_vertex_3d + aperture_top,
				left_vertex_3d + up,
				right_vertex_3d + up,
				right_vertex_3d + aperture_top,
			];

			let uvs = [
				Vec2::new(left_length, aperture_top.y),
				Vec2::new(left_length, room.height),
				Vec2::new(right_uv_start, room.height),
				Vec2::new(right_uv_start, aperture_top.y),
			];

			self.add_convex(verts, uvs, wall.color);
		}
	}

	pub fn build_object(&mut self, object: &Object) {
		match &object.info {
			ObjectInfo::Debug => {
				let forward = object.placement.forward().to_x0y() * 0.1;
				let right = object.placement.right().to_x0y() * 0.1;
				let center = object.placement.position.to_xny(0.3);

				let verts = [
					center + Vec3::from_y(0.2),
					center + forward,
					center - forward/2.0 + right,
					center - forward/2.0 - right,
					center + forward,
				];

				self.add_convex_untextured(verts, Color::magenta());
			}

			ObjectInfo::Ladder{..} => {
				let up = Vec3::from_y(0.7);
				let forward = object.placement.forward().to_x0y();
				let right = object.placement.right().to_x0y();
				let center = object.placement.position.to_x0y();

				let verts = [
					center - forward * 0.08 - right * 0.16,
					center - forward * 0.08 - right * 0.16 + up,
					center - forward * 0.08 + right * 0.16 + up,
					center - forward * 0.08 + right * 0.16,
				];

				self.add_convex_untextured(verts, Color::rgb(0.2, 0.08, 0.02));

				let verts = [
					center - right * 0.2 - forward * 0.2 + Vec3::from_y(0.01),
					center - right * 0.2 + forward * 0.2 + Vec3::from_y(0.01),
					center + right * 0.2 + forward * 0.2 + Vec3::from_y(0.01),
					center + right * 0.2 - forward * 0.2 + Vec3::from_y(0.01),
				];

				self.add_convex_untextured(verts, Color::grey(0.02));
			}

			_ => {}
		}

	}
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