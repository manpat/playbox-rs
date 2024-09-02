use crate::prelude::*;


pub struct Console {
	message_bus: MessageBus,
	visible: bool,

	text_buffer: String,
}

impl Console {
	pub fn new(message_bus: MessageBus) -> Console {
		Console {
			message_bus,
			visible: false,

			text_buffer: String::new(),
		}
	}

	pub fn is_visible(&self) -> bool {
		self.visible
	}

	pub fn update(&mut self, ctx: &mut toybox::Context) {
		if ctx.input.button_just_down(input::keys::F12) {
			self.visible = !self.visible;
		}

		egui::TopBottomPanel::bottom("console-panel")
			.resizable(true)
			.show_separator_line(true)
			.show_animated(&ctx.egui, self.visible, |ui| {
				ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

				// Command line consumes all keyboard input, so we have to ask through egui about input
				if ui.input(|input| input.key_pressed(egui::Key::F12) || input.key_pressed(egui::Key::Escape)) {
					self.visible = false;
				}

				// TODO(pat.m): hook into log and show output history here.

				if let Some(command_str) = show_command_line(ui, &mut self.text_buffer)
					&& let Err(error) = self.process_string(command_str.trim())
				{
					log::error!("{error}");
				}
			});
	}
}

impl Console {
	fn process_string(&mut self, command_str: &str) -> anyhow::Result<()> {
		let (verb, _arguments_str) = command_str.split_once(&[' ', '\t']).unwrap_or((command_str, ""));

		match verb {
			"quit" => self.message_bus.emit(MenuCmd::QuitToDesktop),
			_ => anyhow::bail!("Failed to process command '{verb}'"),
		}

		Ok(())
	}
}




fn show_command_line(ui: &mut egui::Ui, text_buffer: &mut String) -> Option<String> {
	let command_line = egui::TextEdit::singleline(text_buffer)
		.lock_focus(true)
		.clip_text(true)
		.frame(false)
		.desired_width(f32::INFINITY)
		.hint_text("Command time...");

	let response = ui.add(command_line);
	let confirmed = response.lost_focus();
	response.request_focus();

	if confirmed && !text_buffer.trim().is_empty() {
		Some(std::mem::take(text_buffer))
	} else {
		None
	}
}