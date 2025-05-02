use crate::prelude::*;


type CmdFn = Box<dyn Fn(&mut toybox::Context, &str)>;

pub struct Console {
	visible: bool,

	text_buffer: String,
	ready_cmd_str: Option<(String, String)>,

	registered_commands: HashMap<String, CmdFn>,
}

impl Console {
	pub fn new() -> Console {
		Console {
			visible: false,

			text_buffer: String::new(),
			ready_cmd_str: None,

			registered_commands: HashMap::new(),
		}
	}

	pub fn is_visible(&self) -> bool {
		self.visible
	}

	pub fn update(&mut self, ctx: &mut toybox::Context) {
		if let Some((verb, _)) = self.ready_cmd_str.take() {
			log::error!("Failed to process command '{verb}'");
		}

		if ctx.input.button_just_down(input::keys::Backquote) {
			self.visible = !self.visible;
		}

		egui::TopBottomPanel::bottom("console-panel")
			.resizable(true)
			.show_separator_line(true)
			.show_animated(&ctx.egui, self.visible, |ui| {
				ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

				// Command line consumes all keyboard input, so we have to ask through egui about input
				if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Backtick) || input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
					self.visible = false;
					return;
				}

				// TODO(pat.m): hook into log and show output history here.

				if let Some(command_str) = show_command_line(ui, &mut self.text_buffer) {
					let command_str = command_str.trim();
					let (verb, arguments_str) = command_str.split_once(&[' ', '\t']).unwrap_or((command_str, ""));
					if !verb.is_empty() {
						self.ready_cmd_str = Some((verb.to_string(), arguments_str.trim().to_string()));
					}
				}
			});

		if let Some((verb, args)) = self.ready_cmd_str.as_ref()
			&& let Some(cmd_fn) = self.registered_commands.get(verb)
		{
			log::info!("Running command '{verb}'");
			cmd_fn(ctx, &args);
			self.ready_cmd_str = None;
		}
	}

	pub fn register_command(&mut self, verb: impl Into<String>, cmd_fn: impl Fn(&mut toybox::Context, &str) + 'static) {
		use std::collections::hash_map::Entry;

		let cmd_fn = Box::new(cmd_fn);

		match self.registered_commands.entry(verb.into()) {
			Entry::Occupied(mut entry) => {
				log::info!("Replacing already registered console command {}", entry.key());
				let _ = entry.insert(cmd_fn);
			}

			Entry::Vacant(entry) => {
				log::info!("Registered console command '{}'", entry.key());
				entry.insert(cmd_fn);
			}
		}
	}

	pub fn command(&mut self, verb: &str) -> Option<String> {
		if let Some((ready_verb, _)) = &self.ready_cmd_str
			&& ready_verb == verb
		{
			self.ready_cmd_str.take().map(|(_, arguments)| arguments)
		}
		else {
			None
		}
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
	let confirmed = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
	response.request_focus();

	if confirmed && !text_buffer.trim().is_empty() {
		Some(std::mem::take(text_buffer))
	} else {
		None
	}
}



