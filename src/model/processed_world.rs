use crate::prelude::*;
use model::*;

use slotmap::SecondaryMap;

#[derive(Debug)]
pub struct ProcessedWorld {
	wall_infos: SecondaryMap<WallId, WallInfo>,
	room_infos: SecondaryMap<RoomId, RoomInfo>,

	world_change_sub: Subscription<WorldChangedEvent>,
}

impl ProcessedWorld {
	pub fn new(world: &World, message_bus: &MessageBus) -> Self {
		let mut this = Self {
			wall_infos: SecondaryMap::new(),
			room_infos: SecondaryMap::new(),

			world_change_sub: message_bus.subscribe(),
		};

		this.rebuild_world(world);
		this
	}

	pub fn update(&mut self, world: &World, _progress: &ProgressModel, message_bus: &MessageBus) {
		if message_bus.any(&self.world_change_sub) {
			self.rebuild_world(world);
		}
	}

	pub fn wall_info(&self, wall_id: WallId) -> Option<&WallInfo> {
		self.wall_infos.get(wall_id)
	}

	pub fn room_info(&self, room_id: RoomId) -> Option<&RoomInfo> {
		self.room_infos.get(room_id)
	}

	pub fn connection_info(&self, wall_id: WallId) -> Option<&ConnectionInfo> {
		self.wall_infos.get(wall_id)
			.and_then(|wall| wall.connection_info.as_ref())
	}

	pub fn connections_for_room(&self, room_id: RoomId) -> impl Iterator<Item=&'_ ConnectionInfo> + use<'_> {
		let connecting_walls = match self.room_info(room_id) {
			Some(info) => info.connecting_walls.as_slice(),
			None => &[]
		};

		connecting_walls.into_iter()
			.filter_map(move |&wall_id| self.connection_info(wall_id))
	}

	pub fn object_indices_for_room(&self, room_id: RoomId) -> impl Iterator<Item=usize> + use<'_> {
		let object_indices = match self.room_info(room_id) {
			Some(info) => info.object_indices.as_slice(),
			None => &[]
		};

		object_indices.iter().cloned()
	}

	pub fn objects_in_room<'w, 's>(&'s self, room_id: RoomId, world: &'w World) -> impl Iterator<Item=&'w Object> + use<'s, 'w> {
		self.object_indices_for_room(room_id)
			.map(move |idx| &world.objects[idx])
	}

	fn rebuild_world(&mut self, world: &World) {
		self.room_infos.clear();
		self.wall_infos.clear();

		let geometry = &world.geometry;

		for room_id in geometry.rooms.keys() {
			let mut connecting_walls = Vec::new();

			// Collect walls
			for wall_id in geometry.room_walls(room_id) {
				let wall = &geometry.walls[wall_id];

				let connection_info = wall.connected_wall
					.map(|target_id| ConnectionInfo::new(world, wall_id, target_id));

				if connection_info.is_some() {
					connecting_walls.push(wall_id);
				}

				let (start, end) = geometry.wall_vertices(wall_id);
				let direction = (end - start).normalize();
				let normal = direction.perp();

				let wall_info = WallInfo {
					normal,
					connection_info,
				};

				self.wall_infos.insert(wall_id, wall_info);
			}

			// Collect objects
			let object_indices = world.objects.iter().enumerate()
				.filter(|(_, o)| o.placement.room_id == room_id)
				.map(|(index, _)| index)
				.collect();

			self.room_infos.insert(room_id, RoomInfo {
				object_indices,
				connecting_walls,
			});
		}
	}
}




#[derive(Default, Debug)]
pub struct WallInfo {
	// Points out of the room
	pub normal: Vec2,

	pub connection_info: Option<ConnectionInfo>,
}

#[derive(Debug)]
pub struct ConnectionInfo {
	pub target_wall: WallId,
	pub target_room: RoomId,

	pub target_to_source: Mat2x3,
	pub source_to_target: Mat2x3,

	pub yaw_delta: f32,

	pub aperture_start: Vec2,
	pub aperture_end: Vec2,

	// Half width
	pub aperture_extent: f32,

	// Distance from wall center to aperture center
	pub aperture_offset: f32,

	// Height of the aperture
	pub aperture_height: i32,

	// Floor height difference when transitioning connection
	pub height_difference: i32,
}

impl ConnectionInfo {
	fn new(world: &World, source_id: WallId, target_id: WallId) -> Self {
		let geometry = &world.geometry;

		let source_wall = &geometry.walls[source_id];
		let source_room = &geometry.rooms[source_wall.room];

		let target_wall = &geometry.walls[target_id];
		let target_room = &geometry.rooms[target_wall.room];

		let source_wall_length = geometry.wall_length(source_id);
		let target_wall_length = geometry.wall_length(target_id);

		let (start_vertex, end_vertex) = geometry.wall_vertices(source_id);

		let wall_diff = end_vertex - start_vertex;
		let wall_direction = wall_diff / source_wall_length;
		let horizontal_offset = source_wall.horizontal_offset as f32 / 16.0;

		let aperture_extent = source_wall_length.min(target_wall_length) / 2.0;
		let aperture_offset = horizontal_offset.clamp(aperture_extent-source_wall_length/2.0, source_wall_length/2.0-aperture_extent);

		let aperture_center = source_wall_length/2.0 + aperture_offset;

		let aperture_start = start_vertex + wall_direction * (aperture_center - aperture_extent);
		let aperture_end = start_vertex + wall_direction * (aperture_center + aperture_extent);


		let vertical_offset = source_wall.vertical_offset - target_wall.vertical_offset;
		let aperture_height = (source_room.height as i32 - vertical_offset).min(target_room.height as i32 + vertical_offset);

		let target_to_source = calculate_portal_transform(world, source_id, target_id);
		let source_to_target = target_to_source.inverse();

		let yaw_delta = {
			let row = target_to_source.rows[0];
			row.y.atan2(row.x)
		};

		ConnectionInfo {
			target_wall: target_id,
			target_room: target_wall.room,
			target_to_source,
			source_to_target,
			yaw_delta,

			aperture_start,
			aperture_end,

			aperture_extent,
			aperture_offset,

			aperture_height,
			height_difference: vertical_offset,
		}
	}
}



#[derive(Default, Debug)]
pub struct RoomInfo {
	pub object_indices: Vec<usize>,
	pub connecting_walls: Vec<WallId>,
}