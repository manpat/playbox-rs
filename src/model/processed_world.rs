use crate::prelude::*;
use model::*;

use slotmap::SecondaryMap;

#[derive(Debug)]
pub struct ProcessedWorld {
	wall_infos: SecondaryMap<WallId, WallInfo>,
	room_infos: SecondaryMap<RoomId, RoomInfo>,

	geometry: WorldGeometry,

	world_change_sub: Subscription<WorldChangedEvent>,
}

impl ProcessedWorld {
	pub fn new(world: &World, message_bus: &MessageBus) -> Self {
		let mut this = Self {
			wall_infos: SecondaryMap::new(),
			room_infos: SecondaryMap::new(),

			geometry: WorldGeometry::new(),

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

	pub fn geometry(&self) -> &WorldGeometry {
		&self.geometry
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

		self.geometry = world.geometry.clone();
		process_geometry(&mut self.geometry);

		for room_id in self.geometry.rooms.keys() {
			log::debug!("Building {room_id:?}");

			let mut connecting_walls = Vec::new();

			// Collect walls
			for wall_id in self.geometry.room_walls(room_id) {
				log::debug!("--- Collecting {wall_id:?}");
				let connection_info = wall_id.connected_wall(&self.geometry)
					.map(|target_id| ConnectionInfo::new(&self.geometry, wall_id, target_id));

				if connection_info.is_some() {
					connecting_walls.push(wall_id);
				}

				let (start, end) = self.geometry.wall_vertices(wall_id);
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

		log::debug!("Done");
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
	pub aperture_height: f32,

	// Floor height difference when transitioning connection
	pub height_difference: f32,
}

impl ConnectionInfo {
	fn new(geometry: &WorldGeometry, source_id: WallId, target_id: WallId) -> Self {
		let source_wall = &geometry.walls[source_id];
		let source_room = &geometry.rooms[source_wall.room];

		let target_wall = &geometry.walls[target_id];
		let target_room = &geometry.rooms[target_wall.room];

		let source_wall_length = geometry.wall_length(source_id);
		let target_wall_length = geometry.wall_length(target_id);

		let (start_vertex, end_vertex) = geometry.wall_vertices(source_id);

		let wall_diff = end_vertex - start_vertex;
		let wall_direction = wall_diff / source_wall_length;

		let aperture_extent = source_wall_length.min(target_wall_length) / 2.0;
		let aperture_offset = source_wall.horizontal_offset.clamp(aperture_extent-source_wall_length/2.0, source_wall_length/2.0-aperture_extent);

		let aperture_center = source_wall_length/2.0 + aperture_offset;

		let aperture_start = start_vertex + wall_direction * (aperture_center - aperture_extent);
		let aperture_end = start_vertex + wall_direction * (aperture_center + aperture_extent);


		let vertical_offset = source_wall.vertical_offset - target_wall.vertical_offset;
		let aperture_height = (source_room.height - vertical_offset).min(target_room.height + vertical_offset);

		let target_to_source = calculate_portal_transform(geometry, source_id, target_id);
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


fn process_geometry(geometry: &mut WorldGeometry) {
	log::debug!("process_geometry");

	let mut room_queue: SmallVec<[RoomId; 16]> = geometry.rooms.keys().collect();

	'next_room: while let Some(current_room) = room_queue.pop() {
		loop {
			log::debug!("- Starting {current_room:?}");

			let first_wall = current_room.first_wall(geometry);
			let mut current_wall = first_wall;

			// Check that the whole room is convex.

			let mut counter = 50u32;

			'next_wall: loop {
				let next_wall = current_wall.next_wall(geometry);
				log::debug!("--- Checking {current_wall:?} -> {next_wall:?}");

				counter = counter.checked_sub(1).expect("Overflow!");

				let current_direction = geometry.wall_direction(current_wall);
				let next_direction = geometry.wall_direction(next_wall);

				if current_direction.wedge(next_direction) > 0.0 {
					log::debug!("--- --- {current_wall:?} is NOT FINE");
					break 'next_wall;
				}

				if next_wall == first_wall {
					log::debug!("--- --- room loop complete");
					continue 'next_room;
				}

				current_wall = next_wall;
				log::debug!("--- --- fine, moving to {next_wall:?}");
			}

			// Room is concave, search for the next vertex that would make a convex room.
			let current_wall_end_vertex = current_wall.next_vertex(geometry);
			let start_position = current_wall_end_vertex.position(geometry);
			let current_direction = geometry.wall_direction(current_wall);

			let mut next_wall = current_wall;

			// TODO(pat.m): this checks the same initial vertex multiple times. can do better
			loop {
				next_wall.move_next(geometry);

				log::debug!("--- --- --- Checking {next_wall:?}");

				if next_wall == current_wall.prev_wall(geometry) {
					log::error!("Failed to de-concavify wall {current_wall:?} while processing {current_room:?}. next == prev(current)");
					log::error!("current_wall: {current_wall:?}");
					log::error!("next wall: {next_wall:?}");
					log::error!("{geometry:#?}");
					return;
				}

				let next_position = next_wall.vertex(geometry).position(geometry);

				let next_direction = (next_position - start_position).normalize();
				if current_direction.wedge(next_direction) <= 0.0 {
					break;
				}
			}

			let new_room_first_wall = current_wall.next_wall(geometry);
			let new_room_last_wall = next_wall.prev_wall(geometry);

			// hopefully by this point we've found a valid candidate, so insert a wall bridging the two vertices
			let new_wall_current_room = geometry.walls.insert(WallDef {
				source_vertex: current_wall_end_vertex,
				prev_wall: current_wall,
				next_wall: next_wall,
				room: current_room,
				connected_wall: None,

				.. current_wall.get(geometry).clone()
			});

			log::debug!("--- --- Inserted {new_wall_current_room:?}");

			current_wall.get_mut(geometry).next_wall = new_wall_current_room;
			next_wall.get_mut(geometry).prev_wall = new_wall_current_room;
			current_wall_end_vertex.get_mut(geometry).outgoing_wall = new_wall_current_room;

			// Set current rooms first wall to our newly created wall, to avoid the case where it was previously
			// one of the split off walls.
			current_room.get_mut(geometry).first_wall = new_wall_current_room;

			// Split 'lump' into a new room.
			let new_room = geometry.rooms.insert(current_room.get(geometry).clone());

			log::debug!("--- --- Inserted {new_room:?}");

			let new_wall_new_room = geometry.walls.insert(WallDef {
				source_vertex: next_wall.vertex(geometry),
				room: new_room,
				prev_wall: new_room_last_wall,
				next_wall: new_room_first_wall,
				connected_wall: None,

				.. current_wall.get(geometry).clone()
			});

			log::debug!("--- --- Inserted {new_wall_new_room:?}");

			new_room_first_wall.get_mut(geometry).prev_wall = new_wall_new_room;
			new_room_last_wall.get_mut(geometry).next_wall = new_wall_new_room;
			new_room.get_mut(geometry).first_wall = new_wall_new_room;

			// Make sure all walls in new room point to new room
			{
				let mut counter = 50u32;
				let mut wall_it = new_wall_new_room;

				print!("{new_room:?}: {wall_it:?}");

				loop {
					wall_it.move_next(geometry);
					wall_it.get_mut(geometry).room = new_room;

					counter = counter.checked_sub(1).expect("Overflow!");
					if wall_it == new_wall_new_room {
						break
					}

					print!(" -> {wall_it:?}");
				}

				println!();
			}

			// Connect new rooms
			new_wall_new_room.get_mut(geometry).connected_wall = Some(new_wall_current_room);
			new_wall_current_room.get_mut(geometry).connected_wall = Some(new_wall_new_room);

			// Queue new room
			room_queue.push(new_room);
		}
	}

	model::world::validation::validate_geometry(geometry);

	println!("Validated");

	for room_id in geometry.rooms.keys() {
		let mut counter = 50u32;
		let first_wall = room_id.first_wall(geometry);

		let mut wall_it = first_wall;
		print!("{room_id:?}: {wall_it:?}");

		loop {
			wall_it.move_next(geometry);

			counter = counter.checked_sub(1).expect("Overflow!");
			if wall_it == first_wall {
				break
			}

			print!(" -> {wall_it:?}");
		}

		println!();
	}

	log::debug!("Done");
}
