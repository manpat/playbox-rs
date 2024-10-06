use crate::prelude::*;


pub const MAIN_MENU_FRAME_STAGE: gfx::FrameStage = gfx::FrameStage::Ui(10);


pub struct MainMenuScene {
	painter: ui::UiPainter,
}

impl MainMenuScene {
	pub fn new(ctx: &mut Context<'_>) -> anyhow::Result<MainMenuScene> {
		Ok(MainMenuScene{
			painter: ui::UiPainter::new(&mut ctx.gfx, MAIN_MENU_FRAME_STAGE),
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.gfx.frame_encoder.backbuffer_color(Color::rgb(0.05, 0.01, 0.01));

		ctx.input.set_capture_mouse(false);

		let size = ctx.gfx.backbuffer_size().to_vec2()/2.0;
		let screen_rect = Aabb2::new(Vec2::zero(), size);
		let mut content_rect = screen_rect.shrink(8.0); // pad edge

		// Cap size to 150px x 200px
		{
			let Vec2{x, y} = content_rect.size() - Vec2::new(150.0, 200.0);
			content_rect = content_rect.shrink(Vec2::new(x.max(0.0)/2.0, y.max(0.0)/2.0));
		}

		content_rect = content_rect.floor();

		self.painter.rect(content_rect, Color::grey_a(0.0, 0.3));
		let mut builder = self.painter.builder(ctx, ui::DumbLayout::new(content_rect.shrink(8.0)));

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

		self.painter.submit(&mut ctx.gfx, ctx.ui_shared, screen_rect);
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
			painter: ui::UiPainter::new(&mut ctx.gfx, MAIN_MENU_FRAME_STAGE),
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		ctx.input.set_capture_mouse(false);

		let size = ctx.gfx.backbuffer_size().to_vec2()/2.0;
		let screen_rect = Aabb2::new(Vec2::zero(), size);
		let mut content_rect = screen_rect.shrink(8.0); // pad edge

		// Cap size to 200px x 200px
		{
			let Vec2{x, y} = content_rect.size() - Vec2::new(200.0, 200.0);
			content_rect = content_rect.shrink(Vec2::new(x.max(0.0)/2.0, y.max(0.0)/2.0));
		}

		content_rect = content_rect.floor();

		self.painter.rect(content_rect, Color::grey_a(0.0, 0.8));

		let mut builder = self.painter.builder(ctx, ui::DumbLayout::new(content_rect.shrink(8.0)));

		if builder.button("Resume") || ctx.input.button_just_down(input::keys::Escape) {
			ctx.message_bus.emit(MenuCmd::Resume);
		}

		if builder.button("Quit To Menu") {
			ctx.message_bus.emit(MenuCmd::QuitToMain);
		}

		if builder.button("Quit To Desktop") {
			ctx.message_bus.emit(MenuCmd::QuitToDesktop);
		}

		self.painter.submit(&mut ctx.gfx, ctx.ui_shared, screen_rect);
	}
}