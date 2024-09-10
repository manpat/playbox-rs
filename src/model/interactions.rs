use crate::prelude::*;
use model::*;

#[derive(Debug, Default)]
pub struct Interactions {
	pub can_interact: bool,
}

impl Interactions {
	pub fn update(&mut self, player: &Player, world: &World) {
		self.can_interact = false;

		for object in world.objects.iter() {
			if object.placement.room_index != player.placement.room_index {
				continue
			}

			// TODO(pat.m): processed_world.distance(player.placement, object.placement)
			// TODO(pat.m): looking at
			let diff = object.placement.position - player.placement.position;
			if diff.length() < 0.5 {
				self.can_interact = true;
				break
			}
		}
	}
}