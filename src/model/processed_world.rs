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

	pub fn to_processed_placement(&self, source_placement: Placement) -> Placement {
		for room_id in self.to_processed_rooms(source_placement.room_id) {
			if self.geometry.room_contains_point(room_id, source_placement.position) {
				return Placement {
					room_id, .. source_placement
				};
			}
		}

		panic!("Placement doesn't exist in processed world");
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
		if let Err(err) = split_concave_rooms(&mut new_geometry, &mut self.processed_to_source_rooms) {
			log::error!("Failed to process geometry: {err}");
		}

		self.geometry = new_geometry;
		invert_processed_to_source_room_map(&mut self.source_to_processed_rooms, &self.processed_to_source_rooms);

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


fn is_next_vertex_concave(geometry: &WorldGeometry, wall: WallId) -> bool {
	let current_direction = geometry.wall_direction(wall);
	let next_direction = geometry.wall_direction(wall.next_wall(geometry));

	current_direction.wedge(next_direction) > 0.0
}

fn find_concave_wall(geometry: &WorldGeometry, start_wall: WallId) -> Option<WallId> {
	let mut current_wall = start_wall;

	loop {
		// If the next vertex is concave then move on to the next stage.
		if is_next_vertex_concave(geometry, current_wall) {
			return Some(current_wall);
		}

		// If we're back at the first wall then we're done, room is convex.
		let next_wall = current_wall.next_wall(geometry);
		if next_wall == start_wall {
			return None;
		}

		current_wall = next_wall;
	}
}

fn room_is_convex(geometry: &WorldGeometry, room_id: RoomId) -> bool {
	find_concave_wall(geometry, room_id.first_wall(geometry))
		.is_none()
}


fn split_concave_rooms(geometry: &mut WorldGeometry, processed_to_source_rooms: &mut SecondaryMap<RoomId, RoomId>) -> anyhow::Result<()> {
	let mut wall_queue: SmallVec<[WallId; 128]> = geometry.walls.keys()
		.filter(|wall| is_next_vertex_concave(geometry, *wall))
		.collect();

	let mut loop_guard = 1000u32;

	'next_wall: while let Some(start_wall) = wall_queue.pop() {
		if let Some(next) = loop_guard.checked_sub(1) {
			loop_guard = next;
		} else {
			println!("Stuck in a loop");
			println!("{geometry:#?}");

			let walls: Vec<_> = geometry.walls.keys().collect();
			for &wall in walls.iter() {
				let start_vert = wall.vertex(&geometry);
				let end_vert = wall.next_vertex(&geometry);
				let next_concave = find_concave_wall(&geometry, wall);
				println!("{wall:?}: {start_vert:?} -> {end_vert:?}; next concave: {next_concave:?}");
			}

			anyhow::bail!("Stuck in a loop");
		}

		// Check that the whole room is convex.
		let Some(pre_concave_wall) = find_concave_wall(geometry, start_wall) else {
			// Nothing found, check next wall in queue.
			continue 'next_wall
		};

		// Room is concave, search for an appropriate vertex to split off a convex chunk of the current room.
		let current_room = start_wall.room(geometry);
		let mut test_wall = pre_concave_wall.prev_wall(geometry);

		{
			let start_position = pre_concave_wall.next_vertex(geometry).position(geometry);
			let start_direction = geometry.wall_direction(pre_concave_wall);

			// Iterate backwards from the concave vertex looking for the last wall that can be seen fully from the current wall.
			// The vertex from this wall will be the source of the split.
			'find_largest_convex: loop {
				if test_wall == pre_concave_wall.next_wall(geometry) {
					println!("Failed to de-concavify wall {pre_concave_wall:?} while processing {current_room:?} - room fully concave");
					println!("pre_concave_wall: {pre_concave_wall:?}");
					println!("{geometry:#?}");

					let walls: Vec<_> = geometry.walls.keys().collect();
					for &wall in walls.iter() {
						let start_vert = wall.vertex(&geometry);
						let end_vert = wall.next_vertex(&geometry);
						let next_concave = find_concave_wall(&geometry, wall);
						println!("{wall:?}: {start_vert:?} -> {end_vert:?}; next concave: {next_concave:?}");
					}

					anyhow::bail!("Failed to de-concavify wall {pre_concave_wall:?} while processing {current_room:?} - room fully concave");
				}

				let test_wall_source_vertex_offset = test_wall.vertex(geometry).position(geometry) - start_position;
				let wall_crosses_start_vector = start_direction.wedge(test_wall_source_vertex_offset) > 0.0;
				let is_target_vertex_concave = is_next_vertex_concave(geometry, test_wall);

				// TODO(pat.m): check for intermediate intersections!

				if wall_crosses_start_vector || is_target_vertex_concave {
					test_wall.move_next(geometry);

					if test_wall != pre_concave_wall {
						break 'find_largest_convex;
					}

					continue 'next_wall;
				}

				test_wall.move_prev(geometry);
			}
		}

		// Now that we know two vertices we can bridge to make a convex room, start doing that.
		let new_loop_joining_wall = geometry.split_room(test_wall, pre_concave_wall)?;
		let new_room = new_loop_joining_wall.room(geometry);

		// Link new room back to room in original geometry
		if let Some(&source_room) = processed_to_source_rooms.get(current_room) {
			processed_to_source_rooms.insert(new_room, source_room);
		} else {
			processed_to_source_rooms.insert(new_room, current_room);
		}

		// Queue new walls, just in case.
		let old_loop_joining_wall = new_loop_joining_wall.connected_wall(geometry).unwrap();
		wall_queue.push(new_loop_joining_wall);
		wall_queue.push(old_loop_joining_wall);
	}

	if !cfg!(test) {
		model::world::validation::validate_geometry(geometry)?;
	}

	Ok(())
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

	assert!(room_is_convex(&geometry, geometry.first_room()));
	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

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

	assert!(!room_is_convex(&geometry, geometry.first_room()));
	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	model::world::validation::validate_geometry(&geometry).expect("validation failed");

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

	assert!(!room_is_convex(&geometry, geometry.first_room()));
	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	model::world::validation::validate_geometry(&geometry).expect("validation failed");

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

	assert!(!room_is_convex(&geometry, geometry.first_room()));
	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	assert_eq!(room_map.len(), 3);
	assert_eq!(geometry.rooms.len(), 4);
	assert_eq!(geometry.walls.len(), 16);
	assert_eq!(geometry.vertices.len(), 10);

	model::world::validation::validate_geometry(&geometry).expect("validation failed");
}

#[test]
fn split_concave_rooms_with_self_intersecting_geometry_reverse() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	// Some kinda self-intersecting P shape
	geometry.insert_room_from_positions(&[
		Vec2::new(-2.0, 0.0),
		Vec2::new(-2.0, 3.0),
		Vec2::new(-3.0, 3.0),
		Vec2::new(-3.0, 2.0),
		Vec2::new(-0.0, 2.0),
		Vec2::new(-0.0, 1.0),
		Vec2::new(-4.0, 1.0),
		Vec2::new(-4.0, 4.0),
		Vec2::new(-1.0, 4.0),
		Vec2::new(-1.0, 0.0),
	]);

	assert_eq!(geometry.rooms.len(), 1);
	assert_eq!(geometry.walls.len(), 10);
	assert_eq!(geometry.vertices.len(), 10);

	assert!(!room_is_convex(&geometry, geometry.first_room()));
	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	assert_eq!(room_map.len(), 3);
	assert_eq!(geometry.rooms.len(), 4);
	assert_eq!(geometry.walls.len(), 16);
	assert_eq!(geometry.vertices.len(), 10);

	model::world::validation::validate_geometry(&geometry).expect("validation failed");
}

#[test]
fn split_concave_rooms_with_failure_case_1() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	geometry.insert_room_from_positions(&[
		Vec2::new(2.0, 0.0),

		Vec2::new(1.0, 2.0),
		Vec2::new(0.0, 2.0),

		Vec2::new(2.0, 3.0),
		Vec2::new(1.9, 1.0), // This slight concavity breaks the algorithm at time of writing.
	]);

	assert_eq!(geometry.rooms.len(), 1);
	assert_eq!(geometry.walls.len(), 5);
	assert_eq!(geometry.vertices.len(), 5);

	assert!(!room_is_convex(&geometry, geometry.first_room()));

	let walls: Vec<_> = geometry.walls.keys().collect();
	println!("{walls:?}");
	for &wall in walls.iter() {
		let start_vert = wall.vertex(&geometry);
		let end_vert = wall.next_vertex(&geometry);

		let next_concave = find_concave_wall(&geometry, wall);

		println!("{wall:?}: {start_vert:?} -> {end_vert:?}; next concave: {next_concave:?}");
	}

	assert_eq!(find_concave_wall(&geometry, walls[0]), Some(walls[0]));
	assert_eq!(find_concave_wall(&geometry, walls[1]), Some(walls[3]));
	assert_eq!(find_concave_wall(&geometry, walls[2]), Some(walls[3]));
	assert_eq!(find_concave_wall(&geometry, walls[3]), Some(walls[3]));
	assert_eq!(find_concave_wall(&geometry, walls[4]), Some(walls[0]));

	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	model::world::validation::validate_geometry(&geometry).expect("validation failed");
}

#[test]
fn split_concave_rooms_with_failure_case_1_reverse() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	geometry.insert_room_from_positions(&[
		Vec2::new(-1.9, 1.0),
		Vec2::new(-2.0, 3.0),

		Vec2::new(-0.0, 2.0),
		Vec2::new(-1.0, 2.0),

		Vec2::new(-2.0, 0.0),
	]);

	assert_eq!(geometry.rooms.len(), 1);
	assert_eq!(geometry.walls.len(), 5);
	assert_eq!(geometry.vertices.len(), 5);

	assert!(!room_is_convex(&geometry, geometry.first_room()));

	let walls: Vec<_> = geometry.walls.keys().collect();
	println!("{walls:?}");
	for &wall in walls.iter() {
		let start_vert = wall.vertex(&geometry);
		let end_vert = wall.next_vertex(&geometry);

		let next_concave = find_concave_wall(&geometry, wall);

		println!("{wall:?}: {start_vert:?} -> {end_vert:?}; next concave: {next_concave:?}");
	}

	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	model::world::validation::validate_geometry(&geometry).expect("validation failed");
}


#[test]
fn split_concave_rooms_with_failure_case_2() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	geometry.insert_room_from_positions(&[
		Vec2::new(2.0, 0.0),
		Vec2::new(1.5, 3.0),
		Vec2::new(0.0, 4.0),
		Vec2::new(4.0, 4.0),
		Vec2::new(2.5, 3.5),
	]);

	assert!(!room_is_convex(&geometry, geometry.first_room()));

	let walls: Vec<_> = geometry.walls.keys().collect();
	println!("{walls:?}");
	for &wall in walls.iter() {
		let start_vert = wall.vertex(&geometry);
		let end_vert = wall.next_vertex(&geometry);

		let next_concave = find_concave_wall(&geometry, wall);

		println!("{wall:?}: {start_vert:?} -> {end_vert:?}; next concave: {next_concave:?}");
	}

	assert_eq!(find_concave_wall(&geometry, walls[0]), Some(walls[0]));
	assert_eq!(find_concave_wall(&geometry, walls[1]), Some(walls[3]));
	assert_eq!(find_concave_wall(&geometry, walls[2]), Some(walls[3]));
	assert_eq!(find_concave_wall(&geometry, walls[3]), Some(walls[3]));
	assert_eq!(find_concave_wall(&geometry, walls[4]), Some(walls[0]));

	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	model::world::validation::validate_geometry(&geometry).expect("validation failed");
}

#[test]
fn split_concave_rooms_with_failure_case_2_reverse() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	geometry.insert_room_from_positions(&[
		Vec2::new(-2.5, 3.5),
		Vec2::new(-4.0, 4.0),
		Vec2::new(-0.0, 4.0),
		Vec2::new(-1.5, 3.0),
		Vec2::new(-2.0, 0.0),
	]);

	assert!(!room_is_convex(&geometry, geometry.first_room()));

	let walls: Vec<_> = geometry.walls.keys().collect();
	println!("{walls:?}");
	for &wall in walls.iter() {
		let start_vert = wall.vertex(&geometry);
		let end_vert = wall.next_vertex(&geometry);

		let next_concave = find_concave_wall(&geometry, wall);

		println!("{wall:?}: {start_vert:?} -> {end_vert:?}; next concave: {next_concave:?}");
	}

	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	model::world::validation::validate_geometry(&geometry).expect("validation failed");
}


#[test]
fn split_concave_rooms_with_failure_case_3() {
	let mut geometry = WorldGeometry::new();
	let mut room_map = SecondaryMap::new();

	let vertices = [
		Vec2::new(-2.0, -2.0),
		Vec2::new(-2.0, 2.0),
		Vec2::new(2.0, 2.0),
		Vec2::new(2.1721814, -2.002598),
		Vec2::new(-1.1619593, -2.0092072),
		Vec2::new(1.0678722, 0.097930536),
		Vec2::new(0.96893615, -0.5993577),
		Vec2::new(-1.0940874, 0.08872329),
		Vec2::new(-1.5745739, -3.183587),
		Vec2::new(-0.33184546, -1.5009848),
		Vec2::new(2.909154, -2.3008654),
	];

	let vertices: Vec<_> = vertices.into_iter()
		.map(|position| {
			geometry.vertices.insert(VertexDef{ position, outgoing_wall: WallId::default() })
		})
		.collect();

	let room = geometry.rooms.insert(RoomDef::default());

	let walls: Vec<_> = vertices.iter()
		.map(|&source_vertex| geometry.walls.insert(WallDef{ source_vertex, room, ..WallDef::default() }))
		.collect();

	room.get_mut(&mut geometry).first_wall = walls[0];

	let mut set_adjacent = |wall_index: usize, prev, next| {
		let wall_id = walls[wall_index];
		vertices[wall_index].get_mut(&mut geometry).outgoing_wall = wall_id;

		let wall = wall_id.get_mut(&mut geometry);
		wall.next_wall = walls[next];
		wall.prev_wall = walls[prev];
	};

	set_adjacent( 0,  8,  1);
	set_adjacent( 1,  0,  2);
	set_adjacent( 2,  1, 10);
	set_adjacent( 3, 10,  9);
	set_adjacent( 4,  7,  8);
	set_adjacent( 5,  6,  7);
	set_adjacent( 6,  9,  5);
	set_adjacent( 7,  5,  4);
	set_adjacent( 8,  4,  0);
	set_adjacent( 9,  3,  6);
	set_adjacent(10,  2,  3);

	model::world::validation::validate_ids(&geometry).expect("id validation failed");
	model::world::validation::validate_room_loop(&geometry, room).expect("room loop validation failed");

	assert!(!room_is_convex(&geometry, room));

	split_concave_rooms(&mut geometry, &mut room_map).expect("split_concave_rooms failed");

	model::world::validation::validate_geometry(&geometry).expect("validation failed");
}