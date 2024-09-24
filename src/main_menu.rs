use crate::prelude::*;


pub const MAIN_MENU_FRAME_STAGE: gfx::FrameStage = gfx::FrameStage::Ui(10);


pub struct MainMenuScene {
	painter: ui::UiPainter,
}

impl MainMenuScene {
	pub fn new(ctx: &mut Context<'_>) -> anyhow::Result<MainMenuScene> {
		Ok(MainMenuScene{
			painter: ui::UiPainter::new(&mut ctx.gfx, MAIN_MENU_FRAME_STAGE)?,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.gfx.frame_encoder.backbuffer_color(Color::rgb(0.05, 0.01, 0.01));

		ctx.input.set_capture_mouse(false);

		let mut builder = self.painter.builder(ctx);
		let screen_bounds = builder.screen_rect;

		builder.painter.rect(builder.content_rect, Color::grey_a(0.0, 0.3));

		if builder.button("Play") || ctx.input.button_just_down(input::keys::Space) {
			ctx.audio.trigger();
			ctx.message_bus.emit(MenuCmd::Play("default".into()));
		}

		if builder.button("Settings") {
			// ctx.message_bus.emit(MenuCmd::Settings);
		}

		if builder.button("Quit") {
			ctx.message_bus.emit(MenuCmd::QuitToDesktop);
		}

		self.painter.submit(&mut ctx.gfx, screen_bounds);
	}
}


pub enum MenuCmd {
	Play(String),
	Resume,
	Settings,
	QuitToMain,
	QuitToDesktop,
}





pub struct PauseMenuScene {
	painter: ui::UiPainter,
}

impl PauseMenuScene {
	pub fn new(ctx: &mut Context<'_>) -> anyhow::Result<PauseMenuScene> {
		Ok(PauseMenuScene{
			painter: ui::UiPainter::new(&mut ctx.gfx, MAIN_MENU_FRAME_STAGE)?,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.input.set_capture_mouse(false);

		let mut builder = self.painter.builder(ctx);
		let screen_bounds = builder.screen_rect;

		builder.painter.rect(builder.content_rect, Color::grey_a(0.0, 0.8));

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