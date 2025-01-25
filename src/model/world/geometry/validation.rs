use crate::prelude::*;
use model::*;

pub fn validate_geometry(geometry: &WorldGeometry) -> anyhow::Result<()> {
	validate_ids(geometry)?;

	for room_id in geometry.rooms.keys() {
		validate_room_loop(geometry, room_id)?;
		validate_room_convex(geometry, room_id)?;
	}

	for vertex_id in geometry.vertices.keys() {
		validate_vertex(geometry, vertex_id)?;
	}

	Ok(())
}

pub fn validate_ids(geometry: &WorldGeometry) -> anyhow::Result<()> {
	for (room_id, room) in geometry.rooms.iter() {
		anyhow::ensure!(room.first_wall.is_valid(geometry), "{room_id:?} has invalid outgoing wall");
	}

	for (wall_id, wall) in geometry.walls.iter() {
		anyhow::ensure!(wall.source_vertex.is_valid(geometry), "{wall_id:?} has invalid source_vertex");
		anyhow::ensure!(wall.next_wall.is_valid(geometry), "{wall_id:?} has invalid next_wall");
		anyhow::ensure!(wall.prev_wall.is_valid(geometry), "{wall_id:?} has invalid prev_wall");
		anyhow::ensure!(wall.room.is_valid(geometry), "{wall_id:?} has invalid room");

		if let Some(connected_wall) = wall.connected_wall {
			anyhow::ensure!(connected_wall.is_valid(geometry), "{wall_id:?} has invalid connected_wall");
		}
	}

	for (vertex_id, vertex) in geometry.vertices.iter() {
		anyhow::ensure!(vertex.outgoing_wall.is_valid(geometry), "{vertex_id:?} has invalid connected_wall");
	}

	Ok(())
}

pub fn validate_room_convex(geometry: &WorldGeometry, room_id: RoomId) -> anyhow::Result<()> {
	let first_wall = room_id.first_wall(geometry);
	let mut wall_it = first_wall;

	loop {
		let next_wall = wall_it.next_wall(geometry);

		let current_direction = geometry.wall_direction(wall_it);
		let next_direction = geometry.wall_direction(next_wall);

		if current_direction.wedge(next_direction) > 0.0 {
			anyhow::bail!("{wall_it:?} to {next_wall:?} creates a concavity!");
		}

		if next_wall == first_wall {
			return Ok(());
		}

		wall_it = next_wall;
	}
}

pub fn validate_room_loop(geometry: &WorldGeometry, room_id: RoomId) -> anyhow::Result<()> {
	let first_wall = room_id.first_wall(geometry);

	let mut wall_it = first_wall;
	for _ in 0..geometry.walls.len() {
		anyhow::ensure!(wall_it.room(geometry) == room_id, "{wall_it:?} doesn't belong to room {room_id:?}");

		let prev_wall = wall_it;
		wall_it.move_next(geometry);

		anyhow::ensure!(wall_it.prev_wall(geometry) == prev_wall, "{wall_it:?} prev_wall != prev(wall)");

		if wall_it == first_wall {
			return Ok(());
		}
	}

	anyhow::bail!("{room_id:?} does not form a loop!")
}

pub fn validate_vertex(geometry: &WorldGeometry, vertex_id: VertexId) -> anyhow::Result<()> {
	let outgoing_wall = vertex_id.wall(geometry);
	let source_vertex = outgoing_wall.vertex(geometry);
	anyhow::ensure!(source_vertex == vertex_id, "vertex.outgoing({outgoing_wall:?}).source_vertex({source_vertex:?}) != {vertex_id:?}");
	Ok(())
}