use crate::prelude::*;
use model::*;



#[derive(Debug, Clone)]
pub enum HudCmd {
	ShowDialog(()),
	DismissDialog,

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

	hud_cmd: Subscription<HudCmd>,
}

impl HudModel {
	pub fn new(message_bus: &MessageBus) -> Self {
		HudModel {
			in_dialog: false,
			hud_cmd: message_bus.subscribe(),
		}
	}

	pub fn update(&mut self, message_bus: &MessageBus) {
		for msg in message_bus.poll_consume(&self.hud_cmd) {
			match msg {
				HudCmd::DismissDialog => {
					self.in_dialog = false;
				}

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