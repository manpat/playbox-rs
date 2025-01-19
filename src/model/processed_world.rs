use crate::prelude::*;
use model::*;

use slotmap::SecondaryMap;

#[derive(Debug)]
pub struct ProcessedWorld {
	wall_infos: SecondaryMap<WallId, WallInfo>,
	room_infos: SecondaryMap<RoomId, RoomInfo>,

	geometry: WorldGeometry,
	processed_to_source_rooms: SecondaryMap<RoomId, RoomId>,
	source_to_processed_rooms: SecondaryMap<RoomId, SmallVec<[RoomId; 4]>>,

	world_change_sub: Subscription<WorldChangedEvent>,
}

impl ProcessedWorld {
	pub fn new(world: &World, message_bus: &MessageBus) -> Self {
		let mut this = Self {
			wall_infos: SecondaryMap::new(),
			room_infos: SecondaryMap::new(),

			geometry: WorldGeometry::new(),
			processed_to_source_rooms: SecondaryMap::new(),
			source_to_processed_rooms: SecondaryMap::new(),

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

	pub fn to_source_placement(&self, processed_placement: Placement) -> Placement {
		Placement {
			room_id: self.to_source_room(processed_placement.room_id),
			.. processed_placement
		}
	}

	pub fn to_source_room(&self, processed_room_id: RoomId) -> RoomId {
		assert!(processed_room_id.is_valid(&self.geometry), "RoomId given to to_source_room that doesn't exist in processed geometry");

		// Any rooms not in the map but are in the processed geometry already existed in the original geometry.
		self.processed_to_source_rooms.get(processed_room_id)
			.cloned()
			.unwrap_or(processed_room_id)
	}

	pub fn to_processed_rooms(&self, source_room_id: RoomId) -> SmallVec<[RoomId; 4]> {
		let mut rooms = self.source_to_processed_rooms.get(source_room_id).cloned().unwrap_or_default();
		rooms.push(source_room_id);
		rooms
	}

	fn rebuild_world(&mut self, world: &World) {
		self.room_infos.clear();
		self.wall_infos.clear();
		self.processed_to_source_rooms.clear();
		self.source_to_processed_rooms.clear();

		let mut new_geometry = world.geometry.clone();
		if split_concave_rooms(&mut new_geometry, &mut self.processed_to_source_rooms) {
			self.geometry = new_geometry;

			invert_processed_to_source_room_map(&mut self.source_to_processed_rooms, &self.processed_to_source_rooms);

		} else {
			log::error!("Failed to process geometry");
			self.geometry = world.geometry.clone();
			self.processed_to_source_rooms.clear();
		}

		for room_id in self.geometry.rooms.keys() {
			let mut connecting_walls = Vec::new();

			// Collect walls
			for wall_id in self.geometry.room_walls(room_id) {
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


fn split_concave_rooms(geometry: &mut WorldGeometry, processed_to_source_rooms: &mut SecondaryMap<RoomId, RoomId>) -> bool {
	let mut room_queue: SmallVec<[(RoomId, Option<WallId>); 16]> = geometry.rooms.keys()
		.map(|room| (room, None))
		.collect();

	'next_room: while let Some((current_room, maybe_start_wall)) = room_queue.pop() {
		// Retry each room until it is completely cut up into smaller convex rooms.
		loop {
			let start_wall = maybe_start_wall.unwrap_or_else(|| current_room.first_wall(geometry));
			let mut current_wall = start_wall;

			// Check that the whole room is convex.
			'next_wall: loop {
				let next_wall = current_wall.next_wall(geometry);
				let current_direction = geometry.wall_direction(current_wall);
				let next_direction = geometry.wall_direction(next_wall);

				// If the next vertex is concave then move on to the next stage.
				if current_direction.wedge(next_direction) > 0.0 {
					break 'next_wall;
				}

				// If we're back at the first wall then we're done, move on to the next room.
				if next_wall == start_wall {
					continue 'next_room;
				}

				current_wall = next_wall;
			}

			// Room is concave, search for an appropriate vertex to split off a convex chunk of the current room.
			let mut last_fully_visible_wall = current_wall.prev_wall(geometry);

			{
				let start_position = current_wall.next_vertex(geometry).position(geometry);
				let current_direction = geometry.wall_direction(current_wall);

				// Iterate backwards from the concave vertex looking for the last wall that can be seen fully from the current wall.
				// The vertex from this wall will be the source of the split.
				'find_largest_convex: loop {
					if last_fully_visible_wall == current_wall.next_wall(geometry) {
						log::error!("Failed to de-concavify wall {current_wall:?} while processing {current_room:?}.");
						log::error!("current_wall: {current_wall:?}");
						log::error!("{geometry:#?}");
						return false;
					}

					let test_vertex = last_fully_visible_wall.vertex(geometry);
					let start_vertex_position = test_vertex.position(geometry);
					let next_direction = (start_vertex_position - start_position).normalize();

					// If the wall start_position->test_vertex would be concave, then undo the last move_prev.
					// If last_fully_visible_wall != current_wall then we can make a new convex room here.
					if current_direction.wedge(next_direction) > 0.0 {
						last_fully_visible_wall.move_next(geometry);

						if last_fully_visible_wall != current_wall {
							break 'find_largest_convex;
						}

						// let next_start_wall = start_wall.next_wall(geometry);
						// if next_start_wall != current_room.first_wall(geometry) {
						// 	// Try again, skipping the first wall to see if we can find a better candidate.
						// 	room_queue.insert(0, (current_room, Some(next_start_wall)));
						// 	continue 'next_room;
						// }

						// At this point, we've (theoretically) tried starting from every wall in the room.
						// If none of them have a candidate wall 

						// TODO(pat.m): its possible that we could just skip this wall and try others first?
						// although I'm not sure in what circumstances this could happen.
						log::error!("Failed to de-concavify wall {current_wall:?} while processing {current_room:?}.");
						log::error!("current_wall: {current_wall:?}");
						log::error!("{geometry:#?}");
						return false;
					}

					last_fully_visible_wall.move_prev(geometry);
				}
			}

			// Now that we know two vertices we can bridge to make a convex room, start doing that.
			let new_room_first_wall = last_fully_visible_wall;
			let new_room_last_wall = current_wall;

			let current_room_first_wall = current_wall.next_wall(geometry);
			let current_room_last_wall = last_fully_visible_wall.prev_wall(geometry);

			let current_room_new_wall_vertex = new_room_first_wall.vertex(geometry);
			let new_room_new_wall_vertex = current_room_first_wall.vertex(geometry);

			let new_wall_current_room = geometry.walls.insert(WallDef {
				source_vertex: current_room_new_wall_vertex,
				prev_wall: current_room_last_wall,
				next_wall: current_room_first_wall,
				room: current_room,
				connected_wall: None,
				.. current_wall.get(geometry).clone()
			});

			current_room_last_wall.get_mut(geometry).next_wall = new_wall_current_room;
			current_room_first_wall.get_mut(geometry).prev_wall = new_wall_current_room;

			// Set current rooms first wall to our newly created wall, to avoid the case where it was previously
			// one of the split off walls.
			current_room.get_mut(geometry).first_wall = new_wall_current_room;

			// Split convex 'chunk' into a new room, with same attributes as current room.
			let new_room = geometry.rooms.insert(current_room.get(geometry).clone());

			let new_wall_new_room = geometry.walls.insert(WallDef {
				source_vertex: new_room_new_wall_vertex,
				room: new_room,
				prev_wall: new_room_last_wall,
				next_wall: new_room_first_wall,
				connected_wall: None,
				.. current_wall.get(geometry).clone()
			});

			new_room_first_wall.get_mut(geometry).prev_wall = new_wall_new_room;
			new_room_last_wall.get_mut(geometry).next_wall = new_wall_new_room;
			new_room.get_mut(geometry).first_wall = new_wall_new_room;

			// Make sure all walls in new room point to it
			{
				let mut wall_it = new_wall_new_room;

				loop {
					wall_it.get_mut(geometry).room = new_room;
					wall_it.move_next(geometry);

					if wall_it == new_wall_new_room {
						break
					}
				}
			}

			// Connect new rooms
			new_wall_new_room.get_mut(geometry).connected_wall = Some(new_wall_current_room);
			new_wall_current_room.get_mut(geometry).connected_wall = Some(new_wall_new_room);

			// Link new room back to room in original geometry
			if let Some(&source_room) = processed_to_source_rooms.get(current_room) {
				processed_to_source_rooms.insert(new_room, source_room);
			} else {
				processed_to_source_rooms.insert(new_room, current_room);
			}

			// Queue new room, just in case.
			room_queue.push((new_room, None));
		}
	}

	if !cfg!(test) {
		model::world::validation::validate_geometry(geometry);
	}

	true
}


fn invert_processed_to_source_room_map(
	source_to_processed_rooms: &mut SecondaryMap<RoomId, SmallVec<[RoomId; 4]>>,
	processed_to_source_rooms: &SecondaryMap<RoomId, RoomId>)
{
	for (processed_id, &source_id) in processed_to_source_rooms.iter() {
		source_to_processed_rooms.entry(source_id).unwrap()
			.or_default()
			.push(processed_id);
	}
}


#[test]
fn split_concave_rooms_noop_for_simple_geometry() {
	let mut geometry = WorldGeometry::new_square(1.0);
	let mut room_map = SecondaryMap::new();

	assert!(split_concave_rooms(&mut geometry, &mut room_map));

	assert!(room_map.is_empty());
	assert_eq!(geometry.rooms.len(), 1);
	assert_eq!(geometry.walls.len(), 4);
	assert_eq!(geometry.vertices.len(), 4);
}

#[test]
fn split_concave_rooms_with_concave_geometry() {
	let mut geometry = WorldGeometry::new_square(1.0);
	let mut room_map = SecondaryMap::new();

	let first_room = geometry.first_room();
	let first_wall = first_room.first_wall(&geometry);

	let new_position = 0.5 * geometry.wall_center(first_wall);
	let _new_wall = geometry.split_wall(first_wall, new_position);

	assert!(split_concave_rooms(&mut geometry, &mut room_map));

	model::world::validation::validate_geometry(&geometry);

	assert_eq!(room_map.len(), 1);
	assert_eq!(geometry.rooms.len(), 2);
	assert_eq!(geometry.walls.len(), 7);
	assert_eq!(geometry.vertices.len(), 5);
}

#[test]
fn split_concave_rooms_with_very_concave_geometry() {
	let mut geometry = WorldGeometry::new_square(1.0);
	let mut room_map = SecondaryMap::new();

	let first_room = geometry.first_room();
	let first_wall = first_room.first_wall(&geometry);
	let second_wall = first_wall.next_wall(&geometry).next_wall(&geometry);

	let new_position = -0.5 * geometry.wall_center(second_wall);
	geometry.split_wall(second_wall, new_position);

	assert!(split_concave_rooms(&mut geometry, &mut room_map));

	model::world::validation::validate_geometry(&geometry);

	assert_eq!(room_map.len(), 2);
	assert_eq!(geometry.rooms.len(), 3);
	assert_eq!(geometry.walls.len(), 9);
	assert_eq!(geometry.vertices.len(), 5);
}

#[test]
fn split_concave_rooms_with_self_intersecting_geometry() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	// Some kinda self-intersecting P shape
	geometry.insert_room_from_positions(&[
		Vec2::new(1.0, 0.0),
		Vec2::new(1.0, 4.0),
		Vec2::new(4.0, 4.0),
		Vec2::new(4.0, 1.0),
		Vec2::new(0.0, 1.0),
		Vec2::new(0.0, 2.0),
		Vec2::new(3.0, 2.0),
		Vec2::new(3.0, 3.0),
		Vec2::new(2.0, 3.0),
		Vec2::new(2.0, 0.0),
	]);

	assert_eq!(geometry.rooms.len(), 1);
	assert_eq!(geometry.walls.len(), 10);
	assert_eq!(geometry.vertices.len(), 10);

	assert!(split_concave_rooms(&mut geometry, &mut room_map));

	assert_eq!(room_map.len(), 3);
	assert_eq!(geometry.rooms.len(), 4);
	assert_eq!(geometry.walls.len(), 16);
	assert_eq!(geometry.vertices.len(), 10);

	model::world::validation::validate_geometry(&geometry);
}

#[test]
fn split_concave_rooms_with_failure_case_1() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	geometry.insert_room_from_positions(&[
		Vec2::new(1.0, 1.5),

		Vec2::new(2.0, 0.0),
		Vec2::new(2.1, 1.0), // This slight concavity breaks the algorithm at time of writing.
		Vec2::new(2.0, 2.0),

		Vec2::new(0.0, 1.5),
	]);

	assert_eq!(geometry.rooms.len(), 1);
	assert_eq!(geometry.walls.len(), 5);
	assert_eq!(geometry.vertices.len(), 5);

	assert!(split_concave_rooms(&mut geometry, &mut room_map));

	// assert_eq!(room_map.len(), 3);
	// assert_eq!(geometry.rooms.len(), 4);
	// assert_eq!(geometry.walls.len(), 16);
	// assert_eq!(geometry.vertices.len(), 10);

	model::world::validation::validate_geometry(&geometry);
}