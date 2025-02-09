use crate::prelude::*;
use model::*;

#[derive(Debug)]
pub struct Interactions {
	pub hovered_object: Option<ObjectId>,

	player_cmd_sub: Subscription<PlayerCmd>,
}

impl Interactions {
	pub fn new(message_bus: &MessageBus) -> Self {
		Interactions {
			hovered_object: None,

			player_cmd_sub: message_bus.subscribe(),
		}
	}

	pub fn update(&mut self, player: &Player, world: &ProcessedWorld, message_bus: &MessageBus) {
		if message_bus.poll(&self.player_cmd_sub).any(|msg| msg == PlayerCmd::Interact) {
			if let Some(object_id) = self.hovered_object
				&& let Some(object) = world.objects.get(object_id)
			{
				log::info!("Interact '{}'", object.name);

				match &object.info {
					ObjectInfo::Debug => {
						// TODO(pat.m): uuuhhhhhh
						message_bus.emit(HudCmd::ShowDialog(()));
					}
					ObjectInfo::Ladder {target_world, target_object} => {
						// TODO(pat.m): make this not a hudcmd?????
						message_bus.emit(HudCmd::TransitionWorld{
							world_name: target_world.clone(),
							object_name: target_object.clone(),
						});
					}

					_ => {}
				}
			}
		}

		self.hovered_object = None;

		for (object_id, object) in world.objects.iter() {
			if object.placement.room_id != player.placement.room_id {
				continue
			}

			// TODO(pat.m): processed_world.distance(player.placement, object.placement)
			let diff = object.placement.position - player.placement.position;
			let distance = diff.length();
			let direction = diff / distance;

			// TODO(pat.m): determine distance and angle based on size of object
			if distance < 0.5 && player.placement.forward().dot(direction) > 0.7071 {
				self.hovered_object = Some(object_id);
				break
			}
		}
	}

	pub fn can_interact(&self) -> bool {
		self.hovered_object.is_some()
	}
}

