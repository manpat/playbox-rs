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
	vertices: Vec<gfx::StandardVertex>,
	indices: Vec<u32>,
	base_vertex: u32,
}

impl<'w> RoomMeshBuilder<'w> {
	pub fn new(world: &'w World, processed_world: &'w ProcessedWorld) -> Self {
		RoomMeshBuilder {
			world,
			processed_world,
			vertices: Vec::new(),
			indices: Vec::new(),
			base_vertex: 0,
		}
	}

	pub fn upload(&self, gfx: &gfx::System, vbo: gfx::BufferName, ebo: gfx::BufferName) {
		gfx.core.upload_immutable_buffer_immediate(vbo, &self.vertices);
		gfx.core.upload_immutable_buffer_immediate(ebo, &self.indices);
	}
}

impl RoomMeshBuilder<'_> {
	fn add_convex_uvs<VS, UVS>(&mut self, vs: VS, uvs: UVS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
			, UVS: IntoIterator<Item=Vec2>
	{
		let vs = vs.into_iter();
		let uvs = uvs.into_iter();
		let start_index = self.vertices.len() as u32 - self.base_vertex;
		let indices = (1..vs.len() as u32 - 1)
			.flat_map(|i| [start_index, start_index + i, start_index + i + 1]);

		let color = color.into();
		let vertices = vs.zip(uvs).map(|(pos, uv)| gfx::StandardVertex::new(pos, uv, color));

		self.vertices.extend(vertices);
		self.indices.extend(indices);
	}

	fn add_convex<VS>(&mut self, vs: VS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
	{
		self.add_convex_uvs(vs, std::iter::repeat(Vec2::zero()), color);
	}

	pub fn build_room(&mut self, room_index: usize) -> RoomMeshInfo {
		self.base_vertex = self.vertices.len() as u32;
		let base_index = self.indices.len() as u32;

		let room = &self.world.rooms[room_index];
		let up = Vec3::from_y(room.height);

		// ASSUME: rooms are always convex
		let floor_verts = room.wall_vertices.iter().map(|&v| v.to_x0y());
		let ceiling_verts = floor_verts.clone().rev().map(|v| v + up);

		// Floor/Ceiling
		self.add_convex(floor_verts, room.floor_color);
		self.add_convex(ceiling_verts, room.ceiling_color);

		// Walls
		for wall_index in 0..room.walls.len() {
			self.build_wall(WallId{room_index, wall_index});
		}

		// Objects
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

		let (start_vertex, end_vertex) = room.wall_vertices(wall_id.wall_index);

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
				Vec2::new(0.0, 0.0) / 8.0,
				Vec2::new(0.0, room.height) / 8.0,
				Vec2::new(1.0, room.height) / 8.0,
				Vec2::new(1.0, 0.0) / 8.0,
			];

			self.add_convex_uvs(verts, uvs, wall.color);

			return
		};

		let opposing_room = &self.world.rooms[connection_info.target_id.room_index];

		let left_vertex_3d = connection_info.aperture_start.to_x0y();
		let right_vertex_3d = connection_info.aperture_end.to_x0y();

		// Add left and right room height quads
		let verts = [
			start_vertex_3d,
			start_vertex_3d + up,
			left_vertex_3d + up,
			left_vertex_3d,
		];

		self.add_convex(verts, wall.color);

		let verts = [
			right_vertex_3d,
			right_vertex_3d + up,
			end_vertex_3d + up,
			end_vertex_3d,
		];

		self.add_convex(verts, wall.color);

		// Add quads above and below the aperture
		if connection_info.height_difference > 0.0 {
			let aperture_bottom = Vec3::from_y(connection_info.height_difference);

			let verts = [
				left_vertex_3d,
				left_vertex_3d + aperture_bottom,
				right_vertex_3d + aperture_bottom,
				right_vertex_3d,
			];

			self.add_convex(verts, wall.color);
		}

		if connection_info.height_difference + opposing_room.height < room.height {
			let aperture_top = Vec3::from_y(connection_info.height_difference + opposing_room.height);

			let verts = [
				left_vertex_3d + aperture_top,
				left_vertex_3d + up,
				right_vertex_3d + up,
				right_vertex_3d + aperture_top,
			];

			self.add_convex(verts, wall.color);
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

				self.add_convex(verts, Color::magenta());
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

				self.add_convex(verts, Color::rgb(0.2, 0.08, 0.02));

				let verts = [
					center - right * 0.2 - forward * 0.2 + Vec3::from_y(0.01),
					center - right * 0.2 + forward * 0.2 + Vec3::from_y(0.01),
					center + right * 0.2 + forward * 0.2 + Vec3::from_y(0.01),
					center + right * 0.2 - forward * 0.2 + Vec3::from_y(0.01),
				];

				self.add_convex(verts, Color::grey(0.02));
			}

			_ => {}
		}

	}
}
