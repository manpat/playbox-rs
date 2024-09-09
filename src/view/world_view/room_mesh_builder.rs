use crate::prelude::*;
use model::{World, WallId};


pub struct RoomMeshInfo {
	pub base_vertex: u32,
	pub base_index: u32,
	pub num_elements: u32,
}

pub struct RoomMeshBuilder<'w> {
	world: &'w World,
	vertices: Vec<gfx::StandardVertex>,
	indices: Vec<u32>,
	base_vertex: u32,
}

impl<'w> RoomMeshBuilder<'w> {
	pub fn new(world: &'w World) -> Self {
		RoomMeshBuilder {
			world,
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
	fn add_convex<VS>(&mut self, vs: VS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
	{
		let vs = vs.into_iter();
		let start_index = self.vertices.len() as u32 - self.base_vertex;
		let indices = (1..vs.len() as u32 - 1)
			.flat_map(|i| [start_index, start_index + i, start_index + i + 1]);

		let color = color.into();
		let vertices = vs.map(|pos| gfx::StandardVertex::new(pos, Vec2::zero(), color));

		self.vertices.extend(vertices);
		self.indices.extend(indices);
	}

	pub fn build_room(&mut self, room_index: usize) -> RoomMeshInfo {
		self.base_vertex = self.vertices.len() as u32;
		let base_index = self.indices.len() as u32;

		let room = &self.world.rooms[room_index];
		let up = Vec3::from_y(room.height);

		let floor_verts = room.wall_vertices.iter().map(|&v| v.to_x0y());
		let ceiling_verts = floor_verts.clone().rev().map(|v| v + up);

		// Floor/Ceiling
		self.add_convex(floor_verts, room.floor_color);
		self.add_convex(ceiling_verts, room.ceiling_color);

		// Walls
		for wall_index in 0..room.walls.len() {
			let wall_id = WallId{room_index, wall_index};
			let connection = self.world.wall_target(wall_id);

			self.build_wall(wall_id, connection);
		}

		let num_elements = self.indices.len() as u32 - base_index;

		RoomMeshInfo {base_vertex: self.base_vertex, base_index, num_elements}
	}

	pub fn build_wall(&mut self, WallId{room_index, wall_index}: WallId, opposing_wall_id: Option<WallId>) {
		let room = &self.world.rooms[room_index];

		let wall = &room.walls[wall_index];
		let (start_vertex, end_vertex) = room.wall_vertices(wall_index);

		let start_vertex_3d = start_vertex.to_x0y();
		let end_vertex_3d = end_vertex.to_x0y();

		let up = Vec3::from_y(room.height);

		if let Some(opposing_wall_id) = opposing_wall_id {
			// Connected walls may be different lengths, so we need to calculate the aperture that we can actually
			// pass through.
			let opposing_room = &self.world.rooms[opposing_wall_id.room_index];
			let opposing_wall = &opposing_room.walls[opposing_wall_id.wall_index];

			let opposing_wall_length = {
				let (wall_start, wall_end) = opposing_room.wall_vertices(opposing_wall_id.wall_index);
				(wall_end - wall_start).length()
			};

			let wall_length = (end_vertex - start_vertex).length();
			let wall_dir = (end_vertex - start_vertex) / wall_length;

			let aperture_extent = wall_length.min(opposing_wall_length) / 2.0;
			let wall_half_size = wall_length/2.0;

			let aperture_center = wall_half_size + wall.horizontal_offset.clamp(aperture_extent-wall_half_size, wall_half_size-aperture_extent);

			let left_vertex = start_vertex + wall_dir * (aperture_center - aperture_extent);
			let right_vertex = start_vertex + wall_dir * (aperture_center + aperture_extent);

			let left_vertex_3d = left_vertex.to_x0y();
			let right_vertex_3d = right_vertex.to_x0y();

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
			let vertical_offset = wall.vertical_offset - opposing_wall.vertical_offset;
			if vertical_offset > 0.0 {
				let aperture_bottom = Vec3::from_y(vertical_offset);

				let verts = [
					left_vertex_3d,
					left_vertex_3d + aperture_bottom,
					right_vertex_3d + aperture_bottom,
					right_vertex_3d,
				];

				self.add_convex(verts, wall.color);
			}

			if vertical_offset + opposing_room.height < room.height {
				let aperture_top = Vec3::from_y(vertical_offset + opposing_room.height);

				let verts = [
					left_vertex_3d + aperture_top,
					left_vertex_3d + up,
					right_vertex_3d + up,
					right_vertex_3d + aperture_top,
				];

				self.add_convex(verts, wall.color);
			}

		} else {
			let verts = [
				start_vertex_3d,
				start_vertex_3d + up,
				end_vertex_3d + up,
				end_vertex_3d,
			];

			self.add_convex(verts, wall.color);
		}
	}
}
