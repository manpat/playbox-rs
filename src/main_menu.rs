use crate::prelude::*;

pub struct MainMenuScene {
	painter: MenuPainter,
	audio: MyAudioSystem,
}

impl MainMenuScene {
	pub fn new(ctx: &mut toybox::Context, audio: MyAudioSystem) -> anyhow::Result<MainMenuScene> {
		Ok(MainMenuScene{
			painter: MenuPainter::new(&mut ctx.gfx)?,
			audio,
		})
	}

	pub fn update(&mut self, ctx: &mut toybox::Context) {
		ctx.gfx.frame_encoder.backbuffer_color(Color::light_cyan());

		let mut builder = MenuBuilder::new(&mut self.painter, ctx);

		let screen_bounds = builder.bounds;
		let rect = screen_bounds.shrink(Vec2::splat(8.0));

		builder.painter.shape_layer.draw_quad(
			rect,
			Aabb2::new(Vec2::zero(), Vec2::one()),
			Color::grey_a(0.0, 0.3));

		// let mut button_pos = rect.min_max_corner() + Vec2::new(64.0 + 8.0, -16.0 - 8.0);

		if builder.button(rect.center()) {
			self.audio.trigger();
		}

		// button_pos -= Vec2::from_y(32.0 + 8.0);

		// if builder.button(button_pos) {
		// 	self.audio.trigger();
		// }


		self.painter.draw(&mut ctx.gfx, screen_bounds);
	}
}



pub struct MenuBuilder<'mp, 'ctx> {
	painter: &'mp mut MenuPainter,
	input: &'ctx input::System,
	bounds: Aabb2,
}

impl<'mp, 'ctx> MenuBuilder<'mp, 'ctx> {
	pub fn new(painter: &'mp mut MenuPainter, ctx: &'ctx mut toybox::Context) -> Self {
		let size = ctx.gfx.backbuffer_size();
		let bounds = Aabb2::new(Vec2::zero(), size.to_vec2());

		MenuBuilder {
			painter,
			input: &ctx.input,
			bounds,
		}
	}

	pub fn button(&mut self, pos: Vec2) -> bool {
		let size = Vec2::new(256.0, 128.0) / 2.0 + Vec2::splat(8.0);
		let bounds = Aabb2::around_point(pos, size);
		let uvs = Aabb2::new(Vec2::zero(), Vec2::zero());

		let is_hovered = self.input.mouse_position_pixels()
			.map(|pos| bounds.contains_point(pos))
			.unwrap_or(false);

		let is_pressed = is_hovered
			&& self.input.button_down(input::MouseButton::Left);

		let color = match (is_pressed, is_hovered) {
			(true, true) => Color::black().with_alpha(0.2),
			(false, true) => Color::black().with_alpha(0.5),
			(_, false) => Color::white(),
		};

		self.painter.shape_layer.draw_quad(bounds, uvs, Color::grey_a(0.1, 0.3));

		fn floor_vec2(Vec2{x,y}: Vec2) -> Vec2 {
			Vec2::new(x.floor(), y.floor())
		}

		self.painter.glyph_atlas.layout(&self.painter.font, 100, "Abcdef WAAAHhhhh", |info, cursor| {
			let glyph_pos = floor_vec2(bounds.min + Vec2::new(8.0 + cursor, 8.0)) + info.offset_px.to_vec2();
			let glyph_size = info.size_px.to_vec2();
			let glyph_rect = Aabb2::new(glyph_pos, glyph_pos + glyph_size);

			self.painter.text_layer.draw_quad(glyph_rect, info.uv_bounds, color);
		});

		self.painter.glyph_atlas.layout(&self.painter.font, 200, "WEH", |info, cursor| {
			let glyph_pos = floor_vec2(bounds.min + Vec2::new(8.0 + cursor, 8.0 + 100.0)) + info.offset_px.to_vec2();
			let glyph_size = info.size_px.to_vec2();
			let glyph_rect = Aabb2::new(glyph_pos, glyph_pos + glyph_size);

			self.painter.text_layer.draw_quad(glyph_rect, info.uv_bounds, color);
		});

		self.painter.glyph_atlas.layout(&self.painter.font, 24, "Text text text text text", |info, cursor| {
			let glyph_pos = floor_vec2(bounds.min + Vec2::new(8.0 + cursor, 8.0 - 24.0)) + info.offset_px.to_vec2();
			let glyph_size = info.size_px.to_vec2();
			let glyph_rect = Aabb2::new(glyph_pos, glyph_pos + glyph_size);

			self.painter.text_layer.draw_quad(glyph_rect, info.uv_bounds, color);
		});

		is_hovered && self.input.button_just_up(input::MouseButton::Left)
	}
}


const TUFFY_DATA: &[u8] = include_bytes!("../resource/fonts/Tuffy.otf");


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

	pub fn draw(&mut self, gfx: &mut gfx::System, bounds: Aabb2) {
		let aspect = gfx.backbuffer_aspect();
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
				.sampled_image(0, self.glyph_atlas.font_atlas, gfx.resource_manager.linear_sampler)
				.blend_mode(gfx::BlendMode::PREMULTIPLIED_DUAL_SOURCE_COVERAGE)
				.depth_test(false);

			self.text_layer.clear();
		}
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

		let key = (ch, font.file_hash(), font_size);
		match self.glyphs.entry(key) {
			Entry::Occupied(entry) => entry.into_mut(),
			Entry::Vacant(slot) => {
				let (metrics, mut data) = font.rasterize(ch, font_size as f32);
				invert_bitmap(&mut data, metrics.width);

				if metrics.width + self.cursor_x > self.atlas_size.x as usize {
					assert!(self.cursor_y + self.current_row_height_px <= self.atlas_size.y as usize, "Font atlas ran out of space!");

					self.cursor_y += self.current_row_height_px;
					self.cursor_x = 0;
					self.current_row_height_px = 0;
				}

				self.current_row_height_px = self.current_row_height_px.max(metrics.height);

				let pos_px = Vec2i::new(self.cursor_x as i32, self.cursor_y as i32);
				let size_px = Vec2i::new(metrics.width as i32, metrics.height as i32);
				let offset_px = Vec2i::new(metrics.xmin, metrics.ymin);

				self.cursor_x += metrics.width;

				let uv_pos = pos_px.to_vec2() / self.atlas_size.to_vec2();
				let uv_size = size_px.to_vec2() / self.atlas_size.to_vec2();
				let uv_bounds = Aabb2::new(uv_pos, uv_pos + uv_size);

				self.to_insert.push(GlyphInsertion {key, pos_px, size_px, data});

				slot.insert(GlyphInfo {
					uv_bounds,
					offset_px,
					size_px,
					advance_px: metrics.advance_width,
				})
			}
		}
	}

	pub fn layout(&mut self, font: &fontdue::Font, font_size: u32, s: &str, mut f: impl FnMut(&GlyphInfo, f32)) {
		let space_width = font.metrics(' ', font_size as f32).advance_width;

		let mut cursor_x = 0.0;

		for ch in s.chars() {
			// Ignore newlines and co
			if ch.is_whitespace() {
				cursor_x += space_width;
				continue;
			}

			if ch.is_control() {
				continue;
			}

			let info = self.get(font, font_size, ch);
			f(info, cursor_x);
			cursor_x += info.advance_px;
		}
	}
}

struct GlyphInsertion {
	key: (char, usize, u32),
	pos_px: Vec2i,
	size_px: Vec2i,
	data: Vec<u8>,
}

pub struct GlyphInfo {
	pub uv_bounds: Aabb2,
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