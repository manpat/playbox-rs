use crate::prelude::*;
use model::*;



#[derive(Debug, Clone)]
pub enum HudCmd {
	ShowDialog(()),

	// TODO(pat.m): why is this here???????
	TransitionWorld {
		world_name: String,
		object_name: String,
		// transition kind
	}
}


#[derive(Debug)]
pub struct HudModel {
	pub in_dialog: bool,

	player_cmd: Subscription<PlayerCmd>,
	hud_cmd: Subscription<HudCmd>,
}

impl HudModel {
	pub fn new(message_bus: &MessageBus) -> Self {
		HudModel {
			in_dialog: false,
			player_cmd: message_bus.subscribe(),
			hud_cmd: message_bus.subscribe(),
		}
	}

	pub fn update(&mut self, message_bus: &MessageBus) {
		if message_bus.poll(&self.player_cmd).any(|msg| msg == PlayerCmd::DismissDialog) {
			self.in_dialog = false;
		}

		for msg in message_bus.poll_consume(&self.hud_cmd) {
			match msg {
				HudCmd::ShowDialog{..} => {
					self.in_dialog = true;
				}

				HudCmd::TransitionWorld{world_name, ..} => {
					message_bus.emit(MenuCmd::Play(world_name));
				}
			}
		}
	}
}