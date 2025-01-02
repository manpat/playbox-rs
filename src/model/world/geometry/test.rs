use crate::prelude::*;
use model::*;
use model::validation::validate_geometry;

#[test]
fn basic_geometry() {
	let mut geometry = WorldGeometry::new_square(1.0);
	validate_geometry(&geometry);

	let first_room = geometry.first_room();
	let num_walls = geometry.room_walls(first_room).count();
	assert_eq!(num_walls, 4);
}

#[test]
fn split_wall() {
	let mut geometry = WorldGeometry::new_square(1.0);

	let first_room = geometry.first_room();
	let first_wall = first_room.first_wall(&geometry);

	let new_position = 1.5 * geometry.wall_center(first_wall);
	let new_wall = geometry.split_wall(first_wall, new_position);

	validate_geometry(&geometry);

	let num_walls = geometry.room_walls(first_room).count();
	assert_eq!(num_walls, 5);

	let num_walls = geometry.room_walls(first_room).rev().count();
	assert_eq!(num_walls, 5);

	assert_eq!(new_wall.prev_wall(&geometry), first_wall);
	assert_eq!(new_wall, first_wall.next_wall(&geometry));
}