use crate::prelude::*;
use std::cell::RefCell;

// const FONT_DATA: &[u8] = include_bytes!("../../resource/fonts/Tuffy.otf");
// const FONT_DATA: &[u8] = include_bytes!("../../resource/fonts/Quicksand-Light.ttf");
const FONT_DATA: &[u8] = include_bytes!("../../resource/fonts/Saga 8.ttf");
// const FONT_DATA: &[u8] = include_bytes!("../../resource/fonts/Outflank 9.ttf");

pub struct UiShared {
	pub font: fontdue::Font,
	pub glyph_atlas: RefCell<GlyphCache>,
}

impl UiShared {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<UiShared> {
		let font = fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
			.map_err(|err| anyhow::anyhow!("{err}"))?;

		Ok(UiShared {
			font,
			glyph_atlas: RefCell::new(GlyphCache::new(gfx)),
		})
	}
}