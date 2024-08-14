use crate::prelude::*;

pub struct MainMenuScene {
	painter: menu::MenuPainter,
	audio: MyAudioSystem,
}

impl MainMenuScene {
	pub fn new(ctx: &mut Context<'_>, audio: MyAudioSystem) -> anyhow::Result<MainMenuScene> {
		Ok(MainMenuScene{
			painter: menu::MenuPainter::new(&mut ctx.gfx)?,
			audio,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.gfx.frame_encoder.backbuffer_color(Color::light_cyan());

		ctx.input.set_capture_mouse(false);
		let play_shortcut_pressed = ctx.input.button_just_down(input::Key::Space);

		let mut builder = self.painter.builder(ctx);
		let screen_bounds = builder.screen_rect;

		builder.painter.rect(builder.content_rect, Color::grey_a(0.0, 0.3));

		if builder.button("Play") || play_shortcut_pressed {
			self.audio.trigger();
			ctx.message_bus.emit(MenuCmd::Play);
		}

		if builder.button("I'm a big long test button and I go AAAAAAAA. lol. lmao? \"!£$%^&{}()[]") {
			self.audio.trigger();
		}
		// if builder.button("Settings") {
		// 	self.audio.trigger();
		// 	ctx.message_bus.emit(MenuCmd::Settings);
		// }

		if builder.button("Quit") {
			ctx.message_bus.emit(MenuCmd::Quit);
		}

		self.painter.submit(&mut ctx.gfx, screen_bounds);
	}
}


pub enum MenuCmd {
	Play,
	Settings,
	ReturnToMain,
	Quit,
}





pub struct PauseMenuScene {
	painter: menu::MenuPainter,
}

impl PauseMenuScene {
	pub fn new(ctx: &mut Context<'_>) -> anyhow::Result<PauseMenuScene> {
		Ok(PauseMenuScene{
			painter: menu::MenuPainter::new(&mut ctx.gfx)?,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.input.set_capture_mouse(false);

		let resume_shortcut_pressed = ctx.input.button_just_down(input::Key::Escape);

		let mut builder = self.painter.builder(ctx);
		let screen_bounds = builder.screen_rect;

		builder.painter.rect(builder.content_rect, Color::grey_a(0.0, 0.3));

		if builder.button("Resume") || resume_shortcut_pressed {
			ctx.message_bus.emit(MenuCmd::Play);
		}

		if builder.button("Quit To Menu") {
			ctx.message_bus.emit(MenuCmd::ReturnToMain);
		}

		if builder.button("Quit To Desktop") {
			ctx.message_bus.emit(MenuCmd::Quit);
		}

		self.painter.submit(&mut ctx.gfx, screen_bounds);
	}
}