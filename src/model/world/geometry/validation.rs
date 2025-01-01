use crate::prelude::*;
use model::*;

pub fn validate_geometry(geometry: &WorldGeometry) {
	for room_id in geometry.rooms.keys() {
		validate_room_loop(geometry, room_id);
	}

	for vertex_id in geometry.vertices.keys() {
		validate_vertex(geometry, vertex_id);
	}
}


pub fn validate_room_loop(geometry: &WorldGeometry, room_id: RoomId) {
	let first_wall = room_id.first_wall(geometry);

	let mut wall_it = first_wall;
	for _ in 0..geometry.walls.len() {
		assert_eq!(wall_it.room(geometry), room_id, "{wall_it:?} doesn't belong to room {room_id:?}");

		let prev_wall = wall_it;
		wall_it.move_next(geometry);

		assert_eq!(wall_it.prev_wall(geometry), prev_wall, "{wall_it:?} prev_wall != prev(wall)");

		if wall_it == first_wall {
			return;
		}
	}

	panic!("{room_id:?} does not form a loop!");
}

pub fn validate_vertex(geometry: &WorldGeometry, vertex_id: VertexId) {
	let outgoing_wall = vertex_id.wall(geometry);
	let source_vertex = outgoing_wall.vertex(geometry);
	assert_eq!(source_vertex, vertex_id, "vertex.outgoing({outgoing_wall:?}).source_vertex({source_vertex:?}) != {vertex_id:?}")
}