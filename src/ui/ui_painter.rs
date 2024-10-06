use crate::prelude::*;
use super::{UiBuilder, UiShared};


// TODO(pat.m): try ab_glyph. variable fonts??


pub struct UiPainter {
	pub shape_layer: UiPainterLayer,
	pub text_layer: UiPainterLayer,

	frame_stage: gfx::FrameStage,
	f_text_shader: gfx::ShaderHandle,
}

impl UiPainter {
	pub fn new(gfx: &mut gfx::System, frame_stage: gfx::FrameStage) -> UiPainter {
		UiPainter {
			shape_layer: UiPainterLayer::new(),
			text_layer: UiPainterLayer::new(),

			frame_stage,
			f_text_shader: gfx.resource_manager.load_fragment_shader("shaders/text.fs.glsl"),
		}
	}

	pub fn submit(&mut self, gfx: &mut gfx::System, ui_shared: &UiShared, bounds: Aabb2) {
		let projection = Mat4::ortho(bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y, -1.0, 1.0);
		let projection = gfx.frame_encoder.upload(&[projection]);

		if !self.shape_layer.is_empty() {
			gfx.frame_encoder.command_group(self.frame_stage)
				.annotate("Menu")
				.draw(gfx::CommonShader::StandardVertex, gfx::CommonShader::FlatTexturedFragment)
				.elements(self.shape_layer.indices.len() as u32)
				.indexed(&self.shape_layer.indices)
				.ssbo(0, &self.shape_layer.vertices)
				.ubo(0, projection)
				.sampled_image(0, gfx::BlankImage::White, gfx::CommonSampler::Nearest)
				.blend_mode(gfx::BlendMode::ALPHA)
				.depth_test(false);

			self.shape_layer.clear();
		}

		if !self.text_layer.is_empty() {
			gfx.frame_encoder.command_group(self.frame_stage)
				.annotate("Menu (Text)")
				.draw(gfx::CommonShader::StandardVertex, self.f_text_shader)
				.elements(self.text_layer.indices.len() as u32)
				.indexed(&self.text_layer.indices)
				.ssbo(0, &self.text_layer.vertices)
				.ubo(0, projection)
				.sampled_image(0, ui_shared.glyph_atlas.borrow().font_atlas, gfx::CommonSampler::Nearest)
				.blend_mode(gfx::BlendMode::PREMULTIPLIED_DUAL_SOURCE_COVERAGE)
				.depth_test(false);

			self.text_layer.clear();
		}
	}

	pub fn with_shared<'p, 's>(&'p mut self, shared: &'s UiShared) -> UiPainterWithShared<'p, 's> {
		UiPainterWithShared {
			painter: self,
			shared,
		}
	}

	pub fn builder<'mp, 'ctx, L: UiLayout>(&'mp mut self, ctx: &'ctx Context<'_>, layout: L) -> UiBuilder<'mp, 'ctx, L> {
		UiBuilder::new(self, ctx, layout)
	}
}

impl UiPainter {
	pub fn rect(&mut self, geom: Aabb2, color: impl Into<Color>) {
		self.shape_layer.draw_quad(geom, Aabb2::zero(), color);
	}
}



pub struct UiPainterWithShared<'p, 's> {
	pub painter: &'p mut UiPainter,
	pub shared: &'s UiShared,
}

impl<'p, 's> UiPainterWithShared<'p, 's> {
	pub fn text(&mut self, origin: Vec2, font_size: u32, s: impl AsRef<str>, color: impl Into<Color>) {
		let origin = origin.floor();
		let color = color.into();

		self.shared.glyph_atlas.borrow_mut().layout(&self.shared.font, font_size, s, |geom_rect, uv_rect| {
			self.painter.text_layer.draw_quad(geom_rect.translate(origin), uv_rect, color);
		});
	}

	pub fn text_rect(&mut self, font_size: u32, s: impl AsRef<str>) -> Aabb2 {
		let mut full = Aabb2::zero();
		self.shared.glyph_atlas.borrow_mut().layout(&self.shared.font, font_size, s, |geom_rect, _| {
			full = full.include_rect(geom_rect);
		});
		full
	}
}

impl<'p, 's> std::ops::Deref for UiPainterWithShared<'p, 's> {
	type Target = UiPainter;

	fn deref(&self) -> &UiPainter {
		self.painter
	}
}

impl<'p, 's> std::ops::DerefMut for UiPainterWithShared<'p, 's> {
	fn deref_mut(&mut self) -> &mut UiPainter {
		self.painter
	}
}




pub struct UiPainterLayer {
	pub vertices: Vec<gfx::StandardVertex>,
	pub indices: Vec<u32>,
}

impl UiPainterLayer {
	pub fn new() -> UiPainterLayer {
		UiPainterLayer {
			vertices: Vec::new(),
			indices: Vec::new(),
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