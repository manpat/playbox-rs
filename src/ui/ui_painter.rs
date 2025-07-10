use crate::prelude::*;
use super::{GlyphCache};


// TODO(pat.m): try ab_glyph. variable fonts??


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UiPaintMode {
	ShapeUntextured,
	Text,
}

pub struct UiPaintCommand {
	pub paint_mode: UiPaintMode,
	// pub texture: gfx::ImageArgument,

	pub vertex_offset: u32,
	pub index_offset: u32,
	pub element_count: u32,
}

pub struct UiPainter {
	pub commands: Vec<UiPaintCommand>,

	pub vertices: Vec<gfx::StandardVertex>,
	pub indices: Vec<u32>,

	pub base_vertex_count: u32,
	pub base_index_count: u32,

	pub paint_mode: UiPaintMode,
}

impl UiPainter {
	pub fn new() -> UiPainter {
		UiPainter {
			commands: Vec::with_capacity(128),
			vertices: Vec::with_capacity(8<<10),
			indices: Vec::with_capacity(12<<10),

			base_vertex_count: 0,
			base_index_count: 0,

			paint_mode: UiPaintMode::ShapeUntextured,
		}
	}

	fn submit(&mut self) {
		let total_element_count = self.indices.len() as u32;
		if total_element_count == self.base_index_count {
			return;
		}

		self.commands.push(UiPaintCommand {
			paint_mode: self.paint_mode,

			vertex_offset: self.base_vertex_count,
			index_offset: self.base_index_count,
			element_count: total_element_count - self.base_index_count,
		});

		self.base_vertex_count = self.vertices.len() as u32;
		self.base_index_count = total_element_count;
	}

	pub fn finish(&mut self) {
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

impl UiPainter {
	pub fn fill_solid_quad(&mut self, geom: Aabb2, color: impl Into<Color>) {
		self.set_paint_mode(UiPaintMode::ShapeUntextured);
		self.add_quad(geom, Aabb2::zero(), color);
	}

	pub fn add_text_quad(&mut self, geom: Aabb2, uvs: Aabb2, color: impl Into<Color>) {
		self.set_paint_mode(UiPaintMode::Text);
		self.add_quad(geom, uvs, color);
	}
}



impl UiPainter {
	pub fn add_quad(&mut self, geom: Aabb2, uvs: Aabb2, color: impl Into<Color>) {
		let start_index = self.vertices.len() as u32 - self.base_vertex_count;
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