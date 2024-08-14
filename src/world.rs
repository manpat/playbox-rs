use crate::prelude::*;

// world is set of rooms, described by walls.
// rooms are connected by wall pairs

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct World {
	pub rooms: Vec<Room>,
	pub connections: Vec<(GlobalWallId, GlobalWallId)>,

	pub fog_color: Color,
}

impl World {
	pub fn new() -> World {
		World {
			rooms: vec![
				Room {
					walls: [Wall{color: Color::grey(0.4)}; 4].into(),
					wall_vertices: vec![
						Vec2::new(-1.0, -1.0),
						Vec2::new(-1.0,  1.0),
						Vec2::new( 1.0,  1.0),
						Vec2::new( 1.0, -1.0),
					],
					floor_color: Color::grey(0.2),
					ceiling_color: Color::grey(0.7),
				},
			],

			connections: vec![],

			fog_color: Color::white(),
		}
	}

	pub fn new_old() -> World {
		World {
			rooms: vec![
				Room {
					walls: [Wall{color: Color::light_red()}; 4].into(),
					wall_vertices: vec![
						Vec2::new(-1.0, -1.0),
						Vec2::new(-1.0,  1.0),
						Vec2::new( 1.0,  1.0),
						Vec2::new( 1.0, -1.0),
					],
					floor_color: Color::red(),
					ceiling_color: Color::grey(0.5),
				},

				Room {
					walls: [Wall{color: Color::light_cyan()}; 4].into(),
					wall_vertices: vec![
						Vec2::new(-2.0, -1.5),
						Vec2::new(-2.0,  1.0),
						Vec2::new( 0.0,  1.0),
						Vec2::new( 0.0, -1.5),
					],
					floor_color: Color::cyan(),
					ceiling_color: Color::white(),
				},

				Room {
					walls: [Wall{color: Color::light_green()}; 4].into(),
					wall_vertices: vec![
						Vec2::new(0.0, 0.0),
						Vec2::new(0.0, 1.0),
						Vec2::new(4.0, 0.5),
						Vec2::new(4.0, 0.0),
					],
					floor_color: Color::green(),
					ceiling_color: Color::grey(0.5),
				},

				Room {
					walls: [Wall{color: Color::light_magenta()}; 4].into(),
					wall_vertices: vec![
						Vec2::new(0.0, 0.0),
						Vec2::new(0.0, 0.8),
						Vec2::new(2.5, 0.8),
						Vec2::new(3.0, 0.0),
					],
					floor_color: Color::magenta(),
					ceiling_color: Color::grey(0.5),
				},
			],

			connections: vec![
				(GlobalWallId{room_index: 0, wall_index: 0}, GlobalWallId{room_index: 1, wall_index: 2}),

				(GlobalWallId{room_index: 0, wall_index: 1}, GlobalWallId{room_index: 1, wall_index: 0}),
				// (GlobalWallId{room_index: 0, wall_index: 2}, GlobalWallId{room_index: 2, wall_index: 2}),

				// (GlobalWallId{room_index: 0, wall_index: 3}, GlobalWallId{room_index: 2, wall_index: 1}),
				// (GlobalWallId{room_index: 2, wall_index: 2}, GlobalWallId{room_index: 1, wall_index: 0}),

				// (GlobalWallId{room_index: 2, wall_index: 1}, GlobalWallId{room_index: 2, wall_index: 3}),

				(GlobalWallId{room_index: 2, wall_index: 2}, GlobalWallId{room_index: 1, wall_index: 3}),
				(GlobalWallId{room_index: 2, wall_index: 0}, GlobalWallId{room_index: 1, wall_index: 1}),

				(GlobalWallId{room_index: 2, wall_index: 1}, GlobalWallId{room_index: 3, wall_index: 0}),
				(GlobalWallId{room_index: 2, wall_index: 3}, GlobalWallId{room_index: 3, wall_index: 2}),
			],

			fog_color: Color::light_magenta(),
		}
	}

	pub fn try_move_by(&self, position: &mut WorldPosition, yaw: Option<&mut f32>, delta: Vec2) {
		if delta.dot(delta) <= 0.00001 {
			return;
		}

		let mover_radius = 0.1;

		let current_room = &self.rooms[position.room_index];
		let mut desired_position = position.local_position + delta;

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
			collide_vertex(&mut desired_position, *vertex, mover_radius);
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
			if wall_penetration + mover_radius < 0.0 {
				continue
			}

			// If the wall ends a long way away then don't continue
			let distance_along_wall = wall_direction.dot(desired_delta_wall_space);
			if distance_along_wall < 0.0 || distance_along_wall >= wall_length {
				continue
			}

			// We have some kind of intersection here - figure out if we need to transition to another room
			// or if we need to slide against the wall
			let wall_id = GlobalWallId{room_index: position.room_index, wall_index};
			if let Some(opposing_wall_id) = self.connections.iter()
				.find_map(|&(a, b)| {
					if a == wall_id {
						Some(b)
					} else if b == wall_id {
						Some(a)
					} else {
						None
					}
				})
			{
				// Connected walls may be different lengths, so we need to calculate the aperture that we can actually
				// pass through.
				let opposing_wall_length = {
					let opposing_room = &self.rooms[opposing_wall_id.room_index];
					let (wall_start, wall_end) = opposing_room.wall_vertices(opposing_wall_id.wall_index);
					(wall_end - wall_start).length()
				};

				let apperture_extent = wall_length.min(opposing_wall_length) / 2.0;

				let wall_center = wall_length/2.0;
				let apperture_a = wall_start + (wall_center - apperture_extent) * wall_direction;
				let apperture_b = wall_start + (wall_center + apperture_extent) * wall_direction;
				let intersection_dist_from_center = (wall_center - distance_along_wall).abs();

				// Collide with the virtual apperture verts
				collide_vertex(&mut desired_position, apperture_a, mover_radius);
				collide_vertex(&mut desired_position, apperture_b, mover_radius);

				// If we're transitioning through the aperture then we need to transition to the opposing room.
				// Otherwise just slide as normal.
				if intersection_dist_from_center < apperture_extent {
					if wall_penetration < 0.0 {
						continue
					}

					let transform = calculate_portal_transform(self, opposing_wall_id, wall_id);

					position.room_index = opposing_wall_id.room_index;
					position.local_position = transform * desired_position;

					// Apply yaw offset
					if let Some(yaw) = yaw {
						let row = transform.rows[0];
						let angle_delta = row.y.atan2(row.x);
						*yaw -= angle_delta;
					}

					// TODO(pat.m): collide with walls in opposing wall as well
					return;
				}
			}

			// Slide along wall
			desired_position -= wall_direction.perp() * (wall_penetration + mover_radius);
		}

		// If we get here, no transitions have happened and desired_position has been adjusted to remove wall collisions
		position.local_position = desired_position;
	}

	pub fn wall_vertices(&self, wall_id: GlobalWallId) -> (Vec2, Vec2) {
		self.rooms[wall_id.room_index]
			.wall_vertices(wall_id.wall_index)
	}
}


#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Room {
	pub walls: Vec<Wall>,
	pub wall_vertices: Vec<Vec2>,
	pub floor_color: Color,
	pub ceiling_color: Color,
}

impl Room {
	pub fn wall_vertices(&self, wall_index: usize) -> (Vec2, Vec2) {
		let end_vertex_idx = (wall_index+1) % self.wall_vertices.len();
		(self.wall_vertices[wall_index], self.wall_vertices[end_vertex_idx])
	}
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Wall {
	pub color: Color,
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



pub struct WorldView {
	room_mesh_infos: Vec<RoomMeshInfo>,
	vbo: gfx::BufferName,
	ebo: gfx::BufferName,

	v_shader: gfx::ShaderHandle,

	pub needs_rebuild: bool,
}

impl WorldView {
	pub fn new(gfx: &mut gfx::System, world: &World) -> anyhow::Result<Self> {
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

			needs_rebuild: false,
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System, sprites: &mut super::Sprites, world: &World, world_position: WorldPosition) {
		// Draw room you're in
		// then for each wall,
		// 	check if it has a neighbouring room, and if so
		// 	calculate transform between connected walls, and build that room,
		// 	using wall intersection to calculate a frustum to cull by
		if self.needs_rebuild {
			self.needs_rebuild = false;

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

fn calculate_portal_transform(world: &World, from: GlobalWallId, to: GlobalWallId) -> Mat2x3 {
	let from_room = &world.rooms[from.room_index];
	let to_room = &world.rooms[to.room_index];

	let (from_wall_start, from_wall_end) = from_room.wall_vertices(from.wall_index);
	let (to_wall_start, to_wall_end) = to_room.wall_vertices(to.wall_index);

	let from_wall_dir = (from_wall_end - from_wall_start).normalize();
	let to_wall_dir = (to_wall_end - to_wall_start).normalize();

	let s = from_wall_dir.wedge(-to_wall_dir);
	let c = from_wall_dir.dot(-to_wall_dir);
	let new_x = Vec2::new(c, -s);
	let new_y = Vec2::new(s, c);

	let from_wall_center = (from_wall_start + from_wall_end) / 2.0;
	let to_wall_center = (to_wall_start + to_wall_end) / 2.0;
	let rotated_to_wall_center = to_wall_center.x * new_x + to_wall_center.y * new_y;
	let translation = from_wall_center - rotated_to_wall_center;

	Mat2x3::from_columns([
		new_x,
		new_y,
		translation,
	])
}



#[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WorldPosition {
	pub room_index: usize,
	pub local_position: Vec2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GlobalWallId {
	pub room_index: usize,
	pub wall_index: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GlobalVertexId {
	pub room_index: usize,
	pub vertex_index: usize,
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







impl World {
	pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
		let path = path.as_ref();

		if let Some(parent_path) = path.parent() {
			std::fs::create_dir_all(parent_path)?;
		}

		let data = serde_json::to_vec_pretty(self)?;
		std::fs::write(path, &data).map_err(Into::into)
	}

	pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<World> {
		let data = std::fs::read(path)?;
		serde_json::from_slice(&data).map_err(Into::into)
	}
}