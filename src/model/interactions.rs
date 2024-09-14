use crate::prelude::*;
use model::*;

#[derive(Debug)]
pub struct Interactions {
	pub hovered_object: Option<usize>,

	player_cmd_sub: Subscription<PlayerCmd>,
}

impl Interactions {
	pub fn new(message_bus: &MessageBus) -> Self {
		Interactions {
			hovered_object: None,

			player_cmd_sub: message_bus.subscribe(),
		}
	}

	pub fn update(&mut self, player: &Player, world: &World, message_bus: &MessageBus) {
		if let Some(object_index) = self.hovered_object
			&& let Some(object) = world.objects.get(object_index)
		{
			if message_bus.poll(&self.player_cmd_sub).any(|msg| msg == PlayerCmd::Interact) {
				log::info!("Interact '{}'", object.name);

				match object.info {
					ObjectInfo::Debug => {
						// TODO(pat.m): uuuhhhhhh
					}

					_ => {}
				}
			}
		}

		self.hovered_object = None;

		for (object_index, object) in world.objects.iter().enumerate() {
			if object.placement.room_index != player.placement.room_index {
				continue
			}

			// TODO(pat.m): processed_world.distance(player.placement, object.placement)
			let diff = object.placement.position - player.placement.position;
			let distance = diff.length();
			let direction = diff / distance;

			// TODO(pat.m): determine distance and angle based on size of object
			if distance < 0.5 && player.placement.forward().dot(direction) > 0.7071 {
				self.hovered_object = Some(object_index);
				break
			}
		}
	}

	pub fn can_interact(&self) -> bool {
		self.hovered_object.is_some()
	}
}

