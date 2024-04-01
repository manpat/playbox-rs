use crate::prelude::*;

use std::collections::HashMap;

pub struct GlyphCache {
	pub font_atlas: gfx::ImageName,
	atlas_size: Vec2i,

	glyphs: HashMap<(char, usize, u32), GlyphInfo>,
	to_insert: Vec<GlyphInsertion>,

	current_row_height_px: usize,
	cursor_x: usize,
	cursor_y: usize,
}

impl GlyphCache {
	pub fn new(gfx: &mut gfx::System) -> GlyphCache {
		let atlas_size = Vec2i::new(2048, 2048);
		let format = gfx::ImageFormat::unorm8();

		let font_atlas = gfx.core.create_image_2d(format, atlas_size);
		gfx.core.set_debug_label(font_atlas, "Glyph Atlas");

		GlyphCache {
			font_atlas,
			atlas_size,
			glyphs: HashMap::new(),
			to_insert: Vec::new(),

			current_row_height_px: 0,
			cursor_x: 0,
			cursor_y: 0,
		}
	}

	pub fn update_atlas(&mut self, gfx: &mut gfx::System) {
		for insertion in self.to_insert.drain(..) {
			let range = gfx::ImageRange::from_2d_range(insertion.pos_px, insertion.size_px);
			gfx.core.upload_image(self.font_atlas, range, gfx::ImageFormat::unorm8(), &insertion.data);
		}
	}

	pub fn get(&mut self, font: &fontdue::Font, font_size: u32, ch: char) -> &GlyphInfo {
		use std::collections::hash_map::Entry;

		let glyph_margin = 1;

		let key = (ch, font.file_hash(), font_size);
		match self.glyphs.entry(key) {
			Entry::Occupied(entry) => entry.into_mut(),
			Entry::Vacant(slot) => {
				let (metrics, mut data) = font.rasterize(ch, font_size as f32);
				invert_bitmap(&mut data, metrics.width);

				if metrics.width + self.cursor_x > self.atlas_size.x as usize {
					assert!(self.cursor_y + self.current_row_height_px <= self.atlas_size.y as usize, "Font atlas ran out of space!");

					self.cursor_y += self.current_row_height_px + glyph_margin;
					self.cursor_x = 0;
					self.current_row_height_px = 0;
				}

				self.current_row_height_px = self.current_row_height_px.max(metrics.height);

				let pos_px = Vec2i::new(self.cursor_x as i32, self.cursor_y as i32);
				let size_px = Vec2i::new(metrics.width as i32, metrics.height as i32);
				let offset_px = Vec2i::new(metrics.xmin, metrics.ymin);

				self.cursor_x += metrics.width + glyph_margin;

				let uv_pos = pos_px.to_vec2() / self.atlas_size.to_vec2();
				let uv_size = size_px.to_vec2() / self.atlas_size.to_vec2();
				let uv_rect = Aabb2::new(uv_pos, uv_pos + uv_size);

				self.to_insert.push(GlyphInsertion {pos_px, size_px, data});

				slot.insert(GlyphInfo {
					uv_rect,
					offset_px,
					size_px,
					advance_px: metrics.advance_width,
				})
			}
		}
	}

	pub fn layout(&mut self, font: &fontdue::Font, font_size: u32, s: &str, mut f: impl FnMut(Aabb2, Aabb2)) {
		use fontdue::layout::{Layout, TextStyle, CoordinateSystem, LayoutSettings, VerticalAlign};
		let mut layout = Layout::new(CoordinateSystem::PositiveYUp);

		layout.reset(&LayoutSettings {
			x: 0.0,
			y: font_size as f32,
			..LayoutSettings::default()
		});
		layout.append(&[font], &TextStyle::new(s, font_size as f32, 0));

		for glyph in layout.glyphs() {
			let ch = glyph.parent;

			if ch.is_whitespace() || ch.is_control() {
				continue;
			}

			let info = self.get(font, font_size, ch);

			let offset = Vec2::new(glyph.x, glyph.y);
			let geom_rect = Aabb2::new(offset, offset + info.size_px.to_vec2());
			f(geom_rect, info.uv_rect);
		}
	}
}

struct GlyphInsertion {
	pos_px: Vec2i,
	size_px: Vec2i,
	data: Vec<u8>,
}

pub struct GlyphInfo {
	pub uv_rect: Aabb2,
	pub offset_px: Vec2i,
	pub size_px: Vec2i,
	pub advance_px: f32,
}

fn invert_bitmap(data: &mut [u8], width: usize) {
	let height = data.len() / width;

	for y in 0..height/2 {
		let inv_y = height - y - 1;

		let line_idx0 = y * width;
		let line_idx1 = inv_y * width;

		for x in 0..width {
			data.swap(line_idx0 + x, line_idx1 + x);
		}
	}
}