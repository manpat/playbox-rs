use toybox::prelude::*;
use gfx::vertex::ColorVertex2D;
use crate::model;

mod srgb;
mod perf;

pub struct DebugView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	vertex_buffer: gfx::Buffer<ColorVertex2D>,
	index_buffer: gfx::Buffer<u16>,

	srgb_view: srgb::SrgbView,
	perf_view: perf::PerfView,
}

impl DebugView {
	pub fn new(gfx: &mut gfx::ResourceContext<'_>, scene: &model::Scene) -> Result<DebugView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_2D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let mut vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer::<ColorVertex2D>(gfx::BufferUsage::Stream);
		let mut index_buffer = gfx.new_buffer(gfx::BufferUsage::Static);

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		let indices = [
			0, 1, 2,
			0, 2, 3,
		];

		index_buffer.upload(&indices);

		Ok(DebugView {
			shader,
			vao,
			vertex_buffer,
			index_buffer,

			srgb_view: srgb::SrgbView::new(gfx, scene)?,
			perf_view: perf::PerfView::new(gfx)?,
		})
	}


	#[instrument(skip_all)]
	pub fn update(&mut self, engine: &toybox::Engine, debug_model: &model::Debug) {
		let vertices = [
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new(-0.02,-0.02), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new( 0.02,-0.02), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new( 0.02, 0.02), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new(-0.02, 0.02), Vec3::new(1.0, 1.0, 1.0)),
		];

		self.vertex_buffer.upload(&vertices);

		self.srgb_view.update();
		self.perf_view.update(&engine.instrumenter, engine.gfx.aspect());
	}


	#[instrument(skip_all)]
	pub fn draw(&self, ctx: &mut super::ViewContext, debug_model: &model::Debug) {
		{
			let _section = ctx.perf.scoped_section("debug");

			ctx.gfx.bind_vao(self.vao);
			ctx.gfx.bind_shader(self.shader);
			ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.index_buffer.len());
		}

		if debug_model.srgb_active {
			self.srgb_view.draw(ctx);
		}

		if debug_model.perf_active {
			self.perf_view.draw(ctx);
		}
	}
}