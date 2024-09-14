use crate::prelude::*;
use model::*;



#[derive(Debug, Clone, Copy)]
pub enum HudCmd {
	ShowDialog(()),
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

		for msg in message_bus.poll(&self.hud_cmd) {
			match msg {
				HudCmd::ShowDialog{..} => {
					self.in_dialog = true;
				}
			}
		}
	}
}