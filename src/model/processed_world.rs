use crate::prelude::*;
use model::*;



#[derive(Debug)]
pub struct ProcessedWorld {
	wall_infos: HashMap<WallId, WallInfo>,
	active_objects: Vec<usize>,

	world_change_sub: Subscription<WorldChangedEvent>,
}

impl ProcessedWorld {
	pub fn new(world: &World, message_bus: &MessageBus) -> Self {
		let mut this = Self {
			wall_infos: HashMap::default(),

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

	pub fn connection_info(&self, wall_id: WallId) -> Option<&ConnectionInfo> {
		self.wall_infos.get(&wall_id)
			.and_then(|wall| wall.connection_info.as_ref())
	}

	pub fn is_object_active(&self, object_index: usize) -> bool {
		self.active_objects.contains(&object_index)
	}

	fn rebuild_world(&mut self, world: &World) {
		self.wall_infos.clear();

		for (room_index, room) in world.rooms.iter().enumerate() {
			for wall_index in 0..room.walls.len() {
				let wall_id = WallId{room_index, wall_index};
				let connection_info = world.wall_target(wall_id)
					.map(|target_id| ConnectionInfo::new(world, wall_id, target_id));

				let direction = world.wall_vector(wall_id).normalize();
				let normal = direction.perp();

				let wall_info = WallInfo {
					normal,
					connection_info,
				};

				self.wall_infos.insert(wall_id, wall_info);
			}
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