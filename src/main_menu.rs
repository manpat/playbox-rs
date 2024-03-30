use toybox::*;
use crate::audio::MyAudioSystem;
use crate::sprites::Sprites;
use crate::toy_draw::ToyRenderer;


pub struct MainMenuScene {

}

impl MainMenuScene {
	pub fn new(ctx: &mut toybox::Context, audio: MyAudioSystem) -> anyhow::Result<MainMenuScene> {
		Ok(MainMenuScene{})
	}

	pub fn update(&mut self, ctx: &mut toybox::Context) {
		ctx.gfx.frame_encoder.backbuffer_color(Color::light_cyan());
	}
}