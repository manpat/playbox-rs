use crate::prelude::*;


pub const MAIN_MENU_FRAME_STAGE: gfx::FrameStage = gfx::FrameStage::Ui(10);


pub struct MainMenuScene {
	painter: menu::MenuPainter,
}

impl MainMenuScene {
	pub fn new(ctx: &mut Context<'_>) -> anyhow::Result<MainMenuScene> {
		Ok(MainMenuScene{
			painter: menu::MenuPainter::new(&mut ctx.gfx, MAIN_MENU_FRAME_STAGE)?,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.gfx.frame_encoder.backbuffer_color(Color::light_cyan());

		ctx.input.set_capture_mouse(false);

		let mut builder = self.painter.builder(ctx);
		let screen_bounds = builder.screen_rect;

		builder.painter.rect(builder.content_rect, Color::grey_a(0.0, 0.3));

		if builder.button("Play") || ctx.input.button_just_down(input::keys::Space) {
			ctx.audio.trigger();
			ctx.message_bus.emit(MenuCmd::Play(()));
		}

		if builder.button("I'm a big long test button and I go AAAAAAAA. lol. lmao? \"!£$%^&{}()[]") {
			ctx.audio.trigger();
		}
		// if builder.button("Settings") {
		// 	ctx.audio.trigger();
		// 	ctx.message_bus.emit(MenuCmd::Settings);
		// }

		if builder.button("Quit") {
			ctx.message_bus.emit(MenuCmd::QuitToDesktop);
		}

		self.painter.submit(&mut ctx.gfx, screen_bounds);
	}
}


pub enum MenuCmd {
	Play(()),
	Resume,
	Settings,
	QuitToMain,
	QuitToDesktop,
}





pub struct PauseMenuScene {
	painter: menu::MenuPainter,
}

impl PauseMenuScene {
	pub fn new(ctx: &mut Context<'_>) -> anyhow::Result<PauseMenuScene> {
		Ok(PauseMenuScene{
			painter: menu::MenuPainter::new(&mut ctx.gfx, MAIN_MENU_FRAME_STAGE)?,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.input.set_capture_mouse(false);

		let mut builder = self.painter.builder(ctx);
		let screen_bounds = builder.screen_rect;

		builder.painter.rect(builder.content_rect, Color::grey_a(0.0, 0.3));

		if builder.button("Resume") || ctx.input.button_just_down(input::keys::Escape) {
			ctx.message_bus.emit(MenuCmd::Resume);
		}

		if builder.button("Quit To Menu") {
			ctx.message_bus.emit(MenuCmd::QuitToMain);
		}

		if builder.button("Quit To Desktop") {
			ctx.message_bus.emit(MenuCmd::QuitToDesktop);
		}

		self.painter.submit(&mut ctx.gfx, screen_bounds);
	}
}