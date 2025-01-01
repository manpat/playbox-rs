use crate::prelude::*;
use model::world::geometry::*;


#[derive(Clone)]
pub struct RoomWallIterator<'g> {
	pub(super) geometry: &'g WorldGeometry,
	pub(super) first_wall: WallId,
	pub(super) last_wall: WallId,
	pub(super) fused: bool,
}

impl Iterator for RoomWallIterator<'_> {
	type Item = WallId;

	fn next(&mut self) -> Option<WallId> {
		if self.fused {
			return None
		}

		if self.first_wall == self.last_wall {
			self.fused = true
		}

		let result = self.first_wall;
		self.first_wall.move_next(self.geometry);
		Some(result)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		if self.fused {
			return (0, Some(0));
		}

		let mut count = 1;
		let mut it = self.first_wall;

		while it != self.last_wall {
			count += 1;
			it.move_next(self.geometry);
		}

		(count, Some(count))
	}
}

impl DoubleEndedIterator for RoomWallIterator<'_> {
	fn next_back(&mut self) -> Option<WallId> {
		if self.fused {
			return None
		}

		if self.first_wall == self.last_wall {
			self.fused = true
		}

		let result = self.last_wall;
		self.last_wall.move_prev(self.geometry);
		Some(result)
	}
}

impl ExactSizeIterator for RoomWallIterator<'_> {}