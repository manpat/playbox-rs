use crate::prelude::*;

pub struct MainMenuScene {
	painter: menu::MenuPainter,
	audio: MyAudioSystem,
}

impl MainMenuScene {
	pub fn new(ctx: &mut toybox::Context, audio: MyAudioSystem) -> anyhow::Result<MainMenuScene> {
		Ok(MainMenuScene{
			painter: menu::MenuPainter::new(&mut ctx.gfx)?,
			audio,
		})
	}

	pub fn update(&mut self, ctx: &mut toybox::Context) -> Option<MainMenuCmd> {
		ctx.gfx.frame_encoder.backbuffer_color(Color::light_cyan());

		let mut builder = self.painter.builder(ctx);
		let screen_bounds = builder.screen_rect;

		builder.painter.rect(builder.content_rect, Color::grey_a(0.0, 0.3));

		let mut cmd = None;

		if builder.button("Play") {
			self.audio.trigger();
			cmd = Some(MainMenuCmd::Play);
		}

		if builder.button("Settings") {
			self.audio.trigger();
			cmd = Some(MainMenuCmd::Settings);
		}

		if builder.button("Quit") {
			self.audio.trigger();
			cmd = Some(MainMenuCmd::Quit);
		}

		self.painter.submit(&mut ctx.gfx, screen_bounds);

		cmd
	}
}


pub enum MainMenuCmd {
	Play,
	Settings,
	Quit,
}