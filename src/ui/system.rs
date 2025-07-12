use crate::prelude::*;

use ui::glyph_cache::GlyphCache;
use ui::widget_tree::WidgetTree;
use ui::ui_painter::UiPainter;

// const FONT_DATA: &[u8] = include_bytes!("../resource/fonts/Tuffy.otf");
// const FONT_DATA: &[u8] = include_bytes!("../resource/fonts/Quicksand-Light.ttf");
const FONT_DATA: &[u8] = include_bytes!("../../resource/fonts/Saga 8.ttf");
// const FONT_DATA: &[u8] = include_bytes!("../resource/fonts/Outflank 9.ttf");

pub struct UiSystem {
	// Font rendering
	pub font: fontdue::Font,
	pub glyph_cache: GlyphCache,

	pub f_text_shader: gfx::ShaderHandle,

	// Storage
	// TODO(pat.m): ...

	// Config
	pub global_scale: f32,
}

impl UiSystem {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<UiSystem> {
		let font = fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
			.map_err(|err| anyhow::anyhow!("{err}"))?;

		Ok(UiSystem {
			font,
			glyph_cache: GlyphCache::new(gfx),

			f_text_shader: gfx.resource_manager.load_fragment_shader("shaders/text.fs.glsl"),

			global_scale: 0.5,
		})
	}

	pub fn update(&mut self, gfx: &mut gfx::System) {
		self.glyph_cache.update_atlas(gfx);
	}
}
