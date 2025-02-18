use crate::prelude::*;
use model::*;

use super::room_renderer::RoomVertex;

pub struct RoomMeshInfo {
	pub base_vertex: u32,
	pub base_index: u32,
	pub num_elements: u32,

	pub base_light: u32,
	pub num_lights: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RoomLight {
	pub local_pos: Vec3,
	pub radius: f32,
	pub color: Vec3,
	pub power: f32,
}

pub struct RoomMeshBuilder<'w> {
	processed_world: &'w ProcessedWorld,
	vertices: Vec<RoomVertex>,
	indices: Vec<u32>,
	base_vertex: u32,
	current_texture_index: u32,

	lights: Vec<RoomLight>,
}

impl<'w> RoomMeshBuilder<'w> {
	pub fn new(processed_world: &'w ProcessedWorld) -> Self {
		RoomMeshBuilder {
			processed_world,
			vertices: Vec::new(),
			indices: Vec::new(),
			base_vertex: 0,
			current_texture_index: 0,

			lights: Vec::new(),
		}
	}

	pub fn set_texture_index(&mut self, texture_index: u32) {
		self.current_texture_index = texture_index;
	}

	pub fn upload(&self, gfx: &gfx::System, vbo: gfx::BufferName, ebo: gfx::BufferName, light_buffer: gfx::BufferName) {
		gfx.core.upload_immutable_buffer_immediate(vbo, &self.vertices);
		gfx.core.upload_immutable_buffer_immediate(ebo, &self.indices);
		gfx.core.upload_immutable_buffer_immediate(light_buffer, &self.lights);
		gfx.core.debug_marker("Uploaded Room Data");
	}
}

impl RoomMeshBuilder<'_> {
	fn add_convex_textured<VS, UVS>(&mut self, vs: VS, uvs: UVS, color: impl Into<Color>, texture_index: u32)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
			, UVS: IntoIterator<Item=Vec2>
	{
		let vs = vs.into_iter();
		let uvs = uvs.into_iter();
		let start_index = self.vertices.len() as u32 - self.base_vertex;
		let indices = (1..vs.len() as u32 - 1)
			.flat_map(|i| [start_index, start_index + i, start_index + i + 1]);

		let color = color.into();
		let vertices = vs.zip(uvs).map(|(pos, uv)| RoomVertex::new(pos, uv, color, texture_index));

		self.vertices.extend(vertices);
		self.indices.extend(indices);
	}

	fn add_convex<VS, UVS>(&mut self, vs: VS, uvs: UVS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
			, UVS: IntoIterator<Item=Vec2>
	{
		self.add_convex_textured(vs, uvs, color, self.current_texture_index);
	}

	fn add_convex_untextured<VS>(&mut self, vs: VS, color: impl Into<Color>)
		where VS: IntoIterator<Item=Vec3, IntoIter: ExactSizeIterator>
	{
		self.add_convex_textured(vs, std::iter::repeat(Vec2::zero()), color, 0);
	}

	fn add_vertical_quad(&mut self, start: Vec2, end: Vec2, bottom: f32, top: f32, color: Color, u_offset: f32) {
		let start_3d = start.to_x0y();
		let end_3d = end.to_x0y();

		let length = (start - end).length();
		let bottom_3d = Vec3::from_y(bottom);
		let top_3d = Vec3::from_y(top);

		let verts = [
			start_3d + bottom_3d,
			start_3d + top_3d,
			end_3d + top_3d,
			end_3d + bottom_3d,
		];

		// TODO(pat.m): ??
		let uvs = [
			Vec2::new(u_offset, bottom),
			Vec2::new(u_offset, top),
			Vec2::new(u_offset + length, top),
			Vec2::new(u_offset + length, bottom),
		];

		self.add_convex(verts, uvs, color);
	}
}

impl RoomMeshBuilder<'_> {
	pub fn build_room(&mut self, room_id: RoomId) -> RoomMeshInfo {
		self.base_vertex = self.vertices.len() as u32;
		let base_index = self.indices.len() as u32;

		let geometry = self.processed_world.geometry();

		let room = &geometry.rooms[room_id];
		let up = Vec3::from_y(room.height);

		let forward_vertices = geometry.room_walls(room_id)
			.map(|id| id.vertex(geometry).position(geometry));
		let backward_vertices = geometry.room_walls(room_id).rev()
			.map(|id| id.vertex(geometry).position(geometry));

		// ASSUME: rooms are always convex
		let floor_verts = forward_vertices.clone().map(|v| v.to_x0y());
		let floor_uvs = forward_vertices;
		let ceiling_verts = backward_vertices.clone().map(|v| v.to_x0y() + up);
		let ceiling_uvs = backward_vertices;

		// Floor/Ceiling
		self.set_texture_index(3);
		self.add_convex(floor_verts, floor_uvs, room.floor_color);
		self.add_convex(ceiling_verts, ceiling_uvs, room.ceiling_color);

		// Walls
		for wall_id in geometry.room_walls(room_id) {
			self.build_wall(wall_id);
		}

		// Objects
		let base_light = self.lights.len() as u32;

		self.set_texture_index(0);
		for object in self.processed_world.objects_in_room(room_id) {
			self.build_object(object);
		}

		// Collect lights in neighboring rooms
		{
			// Maybe instead of trying to do this per room, it would be better to do some kinda scatter-gather type thing.
			// figure out which rooms are visible from each light, and _then_ invert that to get a list of lights visible from each room.
			// Maybe WorldView::build_visibility_graph can be generalised?

			struct QueueEntry {
				room_id: RoomId,
				from_wall: WallId,
				transform: Mat3x4,
				depth: u32,
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

			let mut room_queue = SmallVec::<[QueueEntry; 16]>::new();
			room_queue.extend(self.processed_world.connections_for_room(room_id)
				.map(|connection| QueueEntry {
					room_id: connection.target_room,
					from_wall: connection.target_wall.connected_wall(self.processed_world.geometry()).unwrap(),
					transform: to_transform(connection.target_to_source, connection.height_difference),
					depth: 1,
				}));

			while let Some(entry) = room_queue.pop() {
				// TODO(pat.m): recurse into all rooms touched by light
				for object in self.processed_world.objects_in_room(entry.room_id) {
					let Some(light) = object.as_light() else { continue };

					// TODO(pat.m): range check
					// TODO(pat.m): cull!

					self.lights.push(RoomLight {
						local_pos: entry.transform * object.placement.position.to_xny(light.height),
						radius: light.radius,
						color: light.color.into(),
						power: light.power,
					});
				}

				room_queue.extend(self.processed_world.connections_for_room(entry.room_id)
					.filter(|connection| connection.target_wall != entry.from_wall)
					.filter_map(|connection| Some(QueueEntry {
						room_id: connection.target_room,
						from_wall: connection.target_wall.connected_wall(self.processed_world.geometry()).unwrap(),
						transform: entry.transform * to_transform(connection.target_to_source, connection.height_difference),
						depth: entry.depth.checked_sub(1)?,
					})));
			}
		}

		// TODO(pat.m): check neighboring rooms for nearby lights

		let num_lights = self.lights.len() as u32 - base_light;
		let num_elements = self.indices.len() as u32 - base_index;

		RoomMeshInfo {
			base_vertex: self.base_vertex,
			base_index,
			num_elements,

			base_light,
			num_lights,
		}
	}

	pub fn build_wall(&mut self, wall_id: WallId) {
		let geometry = self.processed_world.geometry();

		let wall = &geometry.walls[wall_id];
		let room = &geometry.rooms[wall.room];

		// self.set_texture_index(wall_id.wall_index as u32 % 2 + 1);
		self.set_texture_index(1);

		let (start_vertex, end_vertex) = geometry.wall_vertices(wall_id);
		let Some(connection_info) = self.processed_world.connection_info(wall_id) else {
			self.add_vertical_quad(start_vertex, end_vertex, 0.0, room.height, wall.color, 0.0);
			return
		};

		let opposing_room = &geometry.rooms[connection_info.target_room];

		let left_uv_start = (start_vertex - connection_info.aperture_start).length();
		let right_uv_start = (start_vertex - connection_info.aperture_end).length();

		// Add left and right room height quads
		self.add_vertical_quad(start_vertex, connection_info.aperture_start, 0.0, room.height, wall.color, 0.0);
		self.add_vertical_quad(connection_info.aperture_end, end_vertex, 0.0, room.height, wall.color, right_uv_start);

		// Add quads above and below the aperture
		if connection_info.height_difference > 0.0 {
			self.add_vertical_quad(connection_info.aperture_start, connection_info.aperture_end, 0.0, connection_info.height_difference, wall.color, left_uv_start);
		}

		if connection_info.height_difference + opposing_room.height < room.height {
			let aperture_top = connection_info.height_difference + opposing_room.height;
			self.add_vertical_quad(connection_info.aperture_start, connection_info.aperture_end, aperture_top, room.height, wall.color, left_uv_start);
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

			&ObjectInfo::Light(LightObject{color, height, radius, power}) => {
				self.lights.push(RoomLight {
					local_pos: object.placement.position.to_xny(height),
					radius,
					color: color.into(),
					power,
				});
			}

			_ => {}
		}
	}
}


