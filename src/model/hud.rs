use crate::prelude::*;
use model::*;



#[derive(Debug, Clone)]
pub enum HudCmd {
	ShowDialog(()),
	ShowText(String),
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
	pub hud_text: Option<HudText>,

	hud_cmd: Subscription<HudCmd>,
}

impl HudModel {
	pub fn new(bus: &MessageBus) -> Self {
		HudModel {
			in_dialog: false,
			hud_text: None,
			hud_cmd: bus.subscribe(),
		}
	}

	pub fn update(&mut self, bus: &MessageBus) {
		if let Some(hud_text) = &mut self.hud_text {
			hud_text.elapsed_visible_time += 1.0 / 60.0;
			if hud_text.elapsed_visible_time > HUD_TEXT_SHOW_TIME {
				self.hud_text = None;
			}
		}

		for msg in bus.poll_consume(&self.hud_cmd) {
			match msg {
				HudCmd::DismissDialog => {
					self.in_dialog = false;
				}

				HudCmd::ShowDialog{..} => {
					self.in_dialog = true;
				}

				HudCmd::ShowText(text) => {
					self.hud_text = Some(HudText {
						text,
						elapsed_visible_time: 0.0,
					});
				}

				HudCmd::TransitionWorld{world_name, ..} => {
					bus.emit(MenuCmd::Play(world_name));
				}
			}
		}
	}
}


pub const HUD_TEXT_SHOW_TIME: f32 = 5.0;
pub const HUD_TEXT_FADE_IN_TIME: f32 = 1.0;
pub const HUD_TEXT_FADE_OUT_TIME: f32 = 1.5;

#[derive(Debug)]
pub struct HudText {
	pub text: String,
	pub elapsed_visible_time: f32,
}





pub fn handle_hud_commands(ctx: &mut Context, _model: &Model) -> anyhow::Result<()> {
	if let Some(text) = ctx.console.command("hudtext") {
		ctx.bus.emit(HudCmd::ShowText(text));
	}

	Ok(())
}