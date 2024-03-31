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

		let mut button_pos = rect.min_max_corner() + Vec2::new(64.0 + 8.0, -16.0 - 8.0);

		if builder.button(button_pos) {
			self.audio.trigger();
		}

		button_pos -= Vec2::from_y(32.0 + 8.0);

		if builder.button(button_pos) {
			self.audio.trigger();
		}


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
		let size = Vec2::new(128.0, 32.0)/2.0;
		let bounds = Aabb2::around_point(pos, size);
		let uvs = Aabb2::new(Vec2::zero(), Vec2::zero());

		let is_hovered = self.input.mouse_position_pixels()
			.map(|pos| bounds.contains_point(pos))
			.unwrap_or(false);

		let is_pressed = is_hovered
			&& self.input.button_down(input::MouseButton::Left);

		let color = match (is_pressed, is_hovered) {
			(true, true) => Color::yellow(),
			(false, true) => Color::magenta(),
			(_, false) => Color::red(),
		};

		self.painter.shape_layer.draw_quad(bounds, uvs, color);
		self.painter.text_layer.draw_quad(bounds.shrink(Vec2::splat(8.0)), uvs, Color::white());

		is_hovered && self.input.button_just_up(input::MouseButton::Left)
	}
}


pub struct MenuPainter {
	pub shape_layer: MenuPainterLayer,
	pub text_layer: MenuPainterLayer,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,
}

impl MenuPainter {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<MenuPainter> {
		Ok(MenuPainter {
			shape_layer: MenuPainterLayer::new(),
			text_layer: MenuPainterLayer::new(),

			v_shader: gfx.resource_manager.standard_vs_shader,
			f_shader: gfx.resource_manager.flat_fs_shader,
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System, bounds: Aabb2) {
		let aspect = gfx.backbuffer_aspect();
		let projection = Mat4::ortho(bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y, -1.0, 1.0);
		let projection = gfx.frame_encoder.upload(&[projection]);

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
				.draw(self.v_shader, self.f_shader)
				.elements(self.text_layer.indices.len() as u32)
				.indexed(&self.text_layer.indices)
				.ssbo(0, &self.text_layer.vertices)
				.ubo(0, projection)
				.sampled_image(0, gfx.resource_manager.blank_white_image, gfx.resource_manager.nearest_sampler)
				.blend_mode(gfx::BlendMode::ALPHA)
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
			gfx::StandardVertex::new(geom.min.extend(0.0), Vec2::new(0.0, 0.0), color),
			gfx::StandardVertex::new(geom.min_max_corner().extend(0.0), Vec2::new(0.0, 1.0), color),
			gfx::StandardVertex::new(geom.max.extend(0.0), Vec2::new(1.0, 1.0), color),
			gfx::StandardVertex::new(geom.max_min_corner().extend(0.0), Vec2::new(1.0, 0.0), color),
		];

		self.vertices.extend_from_slice(&vertices);
		self.indices.extend(indices);
	}
}

