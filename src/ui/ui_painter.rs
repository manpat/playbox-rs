use crate::prelude::*;
use super::{GlyphCache};


// TODO(pat.m): try ab_glyph. variable fonts??


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UiPaintMode {
	ShapeUntextured,
	Text,
}

pub struct UiPainter<'gfx> {
	pub buffer: UiPaintBuffer,
	pub encoder: gfx::CommandGroupEncoder<'gfx>,

	pub f_text_shader: gfx::ShaderHandle,
	pub font_atlas_image: gfx::ImageHandle,

	pub paint_mode: UiPaintMode,
}

impl UiPainter<'_> {
	fn submit(&mut self) {
		if self.buffer.is_empty() {
			return;
		}

		match self.paint_mode {
			UiPaintMode::ShapeUntextured => {
				self.encoder.draw(gfx::CommonShader::StandardVertex, gfx::CommonShader::FlatTexturedFragment)
					.elements(self.buffer.indices.len() as u32)
					.indexed(&self.buffer.indices)
					.ssbo(0, &self.buffer.vertices)
					.sampled_image(0, gfx::BlankImage::White, gfx::CommonSampler::Nearest)
					.blend_mode(gfx::BlendMode::ALPHA)
					.depth_test(false);
			}

			UiPaintMode::Text => {
				self.encoder.draw(gfx::CommonShader::StandardVertex, self.f_text_shader)
					.elements(self.buffer.indices.len() as u32)
					.indexed(&self.buffer.indices)
					.ssbo(0, &self.buffer.vertices)
					.sampled_image(0, self.font_atlas_image, gfx::CommonSampler::Nearest)
					.blend_mode(gfx::BlendMode::PREMULTIPLIED_DUAL_SOURCE_COVERAGE)
					.depth_test(false);
			}
		}

		self.buffer.clear();
	}

	pub fn finish(mut self) {
		self.submit();
	}

	pub fn set_paint_mode(&mut self, mode: UiPaintMode) {
		if mode == self.paint_mode {
			return;
		}

		self.submit();

		self.paint_mode = mode;
	}
}

impl UiPainter<'_> {
	pub fn rect(&mut self, geom: Aabb2, color: impl Into<Color>) {
		self.set_paint_mode(UiPaintMode::ShapeUntextured);
		self.buffer.draw_quad(geom, Aabb2::zero(), color);
	}

	pub fn text_quad(&mut self, geom: Aabb2, uvs: Aabb2, color: impl Into<Color>) {
		self.set_paint_mode(UiPaintMode::Text);
		self.buffer.draw_quad(geom, uvs, color);
	}
}



pub struct UiPaintBuffer {
	pub vertices: Vec<gfx::StandardVertex>,
	pub indices: Vec<u32>,
}

impl UiPaintBuffer {
	pub fn new() -> UiPaintBuffer {
		UiPaintBuffer {
			vertices: Vec::with_capacity(8<<10),
			indices: Vec::with_capacity(12<<10),
		}
	}

	pub fn clear(&mut self) {
		self.vertices.clear();
		self.indices.clear();
	}

	pub fn is_empty(&self) -> bool {
		self.vertices.is_empty()
	}

	pub fn draw_quad(&mut self, geom: Aabb2, uvs: Aabb2, color: impl Into<Color>) {
		let start_index = self.vertices.len() as u32;
		let indices = [0, 1, 2, 0, 2, 3].into_iter().map(|i| i + start_index);

		let color = color.into();

		let vertices = [
			gfx::StandardVertex::new(geom.min.extend(0.0), uvs.min, color),
			gfx::StandardVertex::new(geom.min_max_corner().extend(0.0), uvs.min_max_corner(), color),
			gfx::StandardVertex::new(geom.max.extend(0.0), uvs.max, color),
			gfx::StandardVertex::new(geom.max_min_corner().extend(0.0), uvs.max_min_corner(), color),
		];

		self.vertices.extend_from_slice(&vertices);
		self.indices.extend(indices);
	}
}