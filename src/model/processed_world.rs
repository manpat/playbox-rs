use crate::prelude::*;
use model::*;



#[derive(Debug)]
pub struct ProcessedWorld {
	wall_connections: HashMap<WallId, ConnectionInfo>,

	world_change_sub: Subscription<WorldChangedEvent>,
}

impl ProcessedWorld {
	pub fn new(world: &World, message_bus: &MessageBus) -> Self {
		let mut this = Self {
			wall_connections: HashMap::default(),

			world_change_sub: message_bus.subscribe(),
		};

		this.rebuild(world);
		this
	}

	pub fn update(&mut self, world: &World, _progress: &ProgressModel, message_bus: &MessageBus) {
		if message_bus.any(&self.world_change_sub) {
			self.rebuild(world);
		}
	}

	fn rebuild(&mut self, world: &World) {
		self.wall_connections.clear();

		for (room_index, room) in world.rooms.iter().enumerate() {
			for wall_index in 0..room.walls.len() {
				let wall_id = WallId{room_index, wall_index};

				if let Some(target_id) = world.wall_target(wall_id) {
					self.wall_connections.entry(wall_id)
						.or_insert_with(|| ConnectionInfo::new(world, wall_id, target_id));
				}
			}
		}
	}
}




// #[derive(Default, Debug)]
// pub struct ProcessedWallInfo {
// 	pub connection: Option<ProcessedWallInfo>,
// }

#[derive(Debug)]
pub struct ConnectionInfo {
	pub target_id: WallId,
	pub transform: Mat2x3,
	pub yaw_delta: f32,

	// Half width
	pub aperture_extent: f32,

	// Distance from wall center to aperture center
	pub aperture_offset: f32,

	// Height of the aperture
	pub aperture_height: f32,

	// Floor height difference when transitioning connection
	pub step_size: f32,
}

impl ConnectionInfo {
	fn new(world: &World, source_id: WallId, target_id: WallId) -> Self {
		let source_room = &world.rooms[source_id.room_index];
		let source_wall = &source_room.walls[source_id.wall_index];

		let target_room = &world.rooms[target_id.room_index];
		let target_wall = &target_room.walls[target_id.wall_index];

		let source_wall_length = world.wall_length(source_id);
		let target_wall_length = world.wall_length(target_id);

		let aperture_extent = source_wall_length.min(target_wall_length) / 2.0;
		let aperture_offset = source_wall.horizontal_offset.clamp(aperture_extent-source_wall_length/2.0, source_wall_length/2.0-aperture_extent);

		let vertical_offset = source_wall.vertical_offset - target_wall.vertical_offset;
		let aperture_height = (source_room.height - vertical_offset).min(target_room.height + vertical_offset);

		let transform = calculate_portal_transform(world, source_id, target_id);
		let yaw_delta = {
			let row = transform.rows[0];
			-row.y.atan2(row.x)
		};

		ConnectionInfo {
			target_id,
			transform,
			yaw_delta,

			aperture_extent,
			aperture_offset,

			aperture_height,
			step_size: vertical_offset,
		}
	}
}