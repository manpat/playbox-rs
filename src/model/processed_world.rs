use crate::prelude::*;
use model::*;



#[derive(Debug)]
pub struct ProcessedWorld {
	wall_infos: HashMap<WallId, WallInfo>,
	room_infos: Vec<RoomInfo>,

	active_objects: Vec<usize>,

	world_change_sub: Subscription<WorldChangedEvent>,
}

impl ProcessedWorld {
	pub fn new(world: &World, message_bus: &MessageBus) -> Self {
		let mut this = Self {
			wall_infos: HashMap::default(),
			room_infos: Vec::new(),

			// TODO(pat.m): some objects might be disabled on first spawn - this should take ProgressModel into account
			active_objects: (0..world.objects.len()).collect(),

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
		self.wall_infos.get(&wall_id)
	}

	pub fn room_info(&self, room_index: usize) -> Option<&RoomInfo> {
		self.room_infos.get(room_index)
	}

	pub fn connection_info(&self, wall_id: WallId) -> Option<&ConnectionInfo> {
		self.wall_infos.get(&wall_id)
			.and_then(|wall| wall.connection_info.as_ref())
	}

	pub fn connections_for_room(&self, room_index: usize) -> impl Iterator<Item=&'_ ConnectionInfo> + use<'_> {
		let connecting_walls = match self.room_info(room_index) {
			Some(info) => info.connecting_walls.as_slice(),
			None => &[]
		};

		connecting_walls.into_iter()
			.filter_map(move |&wall_index| self.connection_info(WallId{room_index, wall_index}))
	}

	pub fn object_indices_for_room(&self, room_index: usize) -> impl Iterator<Item=usize> + use<'_> {
		let object_indices = match self.room_info(room_index) {
			Some(info) => info.object_indices.as_slice(),
			None => &[]
		};

		object_indices.iter().cloned()
	}

	pub fn objects_in_room<'w, 's>(&'s self, room_index: usize, world: &'w World) -> impl Iterator<Item=&'w Object> + use<'s, 'w> {
		self.object_indices_for_room(room_index)
			.map(move |idx| &world.objects[idx])
	}

	pub fn is_object_active(&self, object_index: usize) -> bool {
		self.active_objects.contains(&object_index)
	}

	fn rebuild_world(&mut self, world: &World) {
		self.room_infos.clear();
		self.wall_infos.clear();

		for (room_index, room) in world.rooms.iter().enumerate() {
			let mut connecting_walls = Vec::new();

			// Collect walls
			for wall_index in 0..room.walls.len() {
				let wall_id = WallId{room_index, wall_index};
				let connection_info = world.wall_target(wall_id)
					.map(|target_id| ConnectionInfo::new(world, wall_id, target_id));

				if connection_info.is_some() {
					connecting_walls.push(wall_index);
				}

				let direction = world.wall_vector(wall_id).normalize();
				let normal = direction.perp();

				let wall_info = WallInfo {
					normal,
					connection_info,
				};

				self.wall_infos.insert(wall_id, wall_info);
			}

			// Collect objects
			let object_indices = world.objects.iter().enumerate()
				.filter(|(_, o)| o.placement.room_index == room_index)
				.map(|(index, _)| index)
				.collect();

			self.room_infos.push(RoomInfo {
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
	pub target_id: WallId,

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
	pub aperture_height: f32,

	// Floor height difference when transitioning connection
	pub height_difference: f32,
}

impl ConnectionInfo {
	fn new(world: &World, source_id: WallId, target_id: WallId) -> Self {
		let source_room = &world.rooms[source_id.room_index];
		let source_wall = &source_room.walls[source_id.wall_index];

		let target_room = &world.rooms[target_id.room_index];
		let target_wall = &target_room.walls[target_id.wall_index];

		let source_wall_length = world.wall_length(source_id);
		let target_wall_length = world.wall_length(target_id);

		let start_vertex = source_room.wall_vertices[source_id.wall_index];
		let wall_vector = world.wall_vector(source_id);
		let wall_direction = wall_vector / source_wall_length;

		let aperture_extent = source_wall_length.min(target_wall_length) / 2.0;
		let aperture_offset = source_wall.horizontal_offset.clamp(aperture_extent-source_wall_length/2.0, source_wall_length/2.0-aperture_extent);

		let aperture_center = source_wall_length/2.0 + aperture_offset;

		let aperture_start = start_vertex + wall_direction * (aperture_center - aperture_extent);
		let aperture_end = start_vertex + wall_direction * (aperture_center + aperture_extent);


		let vertical_offset = source_wall.vertical_offset - target_wall.vertical_offset;
		let aperture_height = (source_room.height - vertical_offset).min(target_room.height + vertical_offset);

		let target_to_source = calculate_portal_transform(world, source_id, target_id);
		let source_to_target = target_to_source.inverse();

		let yaw_delta = {
			let row = target_to_source.rows[0];
			row.y.atan2(row.x)
		};

		ConnectionInfo {
			target_id,
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
	pub connecting_walls: Vec<usize>,
}