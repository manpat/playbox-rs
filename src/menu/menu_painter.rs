use crate::prelude::*;
use super::{MenuBuilder};

const TUFFY_DATA: &[u8] = include_bytes!("../../resource/fonts/Tuffy.otf");


pub struct MenuPainter {
	pub shape_layer: MenuPainterLayer,
	pub text_layer: MenuPainterLayer,

	pub font: fontdue::Font,
	pub glyph_atlas: GlyphCache,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,
	f_text_shader: gfx::ShaderHandle,
}

impl MenuPainter {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<MenuPainter> {
		let font = fontdue::Font::from_bytes(TUFFY_DATA, fontdue::FontSettings::default())
			.map_err(|err| anyhow::anyhow!("{}", err))?;

		Ok(MenuPainter {
			shape_layer: MenuPainterLayer::new(),
			text_layer: MenuPainterLayer::new(),

			font,
			glyph_atlas: GlyphCache::new(gfx),

			v_shader: gfx.resource_manager.standard_vs_shader,
			f_shader: gfx.resource_manager.flat_fs_shader,
			f_text_shader: gfx.resource_manager.request(gfx::LoadShaderRequest::fragment("shaders/text.fs.glsl")),
		})
	}

	pub fn submit(&mut self, gfx: &mut gfx::System, bounds: Aabb2) {
		let projection = Mat4::ortho(bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y, -1.0, 1.0);
		let projection = gfx.frame_encoder.upload(&[projection]);

		self.glyph_atlas.update_atlas(gfx);

		if !self.shape_layer.is_empty() {
			gfx.frame_encoder.command_group(gfx::FrameStage::Ui(0))
				.annotate("Menu")
				.draw(self.v_shader, self.f_shader)
				.elements(self.shape_layer.indices.len() as u32)
				.indexed(&self.shape_layer.indices)
				.ssbo(0, &self.shape_layer.vertices)
				.ubo(0, projection)
				.sampled_image(0, gfx.resource_manager.blank_white_image, gfx.resource_manager.nearest_sampler)
				.blend_mode(gfx::BlendMode::ALPHA)
				.depth_test(false);

			self.shape_layer.clear();
		}

		if !self.text_layer.is_empty() {
			gfx.frame_encoder.command_group(gfx::FrameStage::Ui(1))
				.annotate("Menu (Text)")
				.draw(self.v_shader, self.f_text_shader)
				.elements(self.text_layer.indices.len() as u32)
				.indexed(&self.text_layer.indices)
				.ssbo(0, &self.text_layer.vertices)
				.ubo(0, projection)
				.sampled_image(0, self.glyph_atlas.font_atlas, gfx.resource_manager.nearest_sampler)
				.blend_mode(gfx::BlendMode::PREMULTIPLIED_DUAL_SOURCE_COVERAGE)
				.depth_test(false);

			self.text_layer.clear();
		}
	}

	pub fn builder<'mp, 'ctx>(&'mp mut self, ctx: &'ctx mut toybox::Context) -> MenuBuilder<'mp, 'ctx> {
		MenuBuilder::new(self, ctx)
	}
}

impl MenuPainter {
	pub fn rect(&mut self, geom: Aabb2, color: impl Into<Color>) {
		self.shape_layer.draw_quad(geom, Aabb2::point(Vec2::zero()), color);
	}

	pub fn text(&mut self, origin: Vec2, font_size: u32, s: &str, color: impl Into<Color>) {
		let origin = origin.floor();
		let color = color.into();

		self.glyph_atlas.layout(&self.font, font_size, s, |geom_rect, uv_rect| {
			self.text_layer.draw_quad(geom_rect.translate(origin), uv_rect, color);
		});
	}

	pub fn text_rect(&mut self, font_size: u32, s: &str) -> Aabb2 {
		let mut full = Aabb2::zero();
		self.glyph_atlas.layout(&self.font, font_size, s, |geom_rect, _| {
			full = full.expand_to_include_rect(geom_rect);
		});
		full
	}
}



pub struct MenuPainterLayer {
	pub vertices: Vec<gfx::StandardVertex>,
	pub indices: Vec<u32>,
}

impl MenuPainterLayer {
	pub fn new() -> MenuPainterLayer {
		MenuPainterLayer {
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