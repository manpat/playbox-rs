use toybox::prelude::*;
use gfx::vertex::ColorVertex2D;

pub struct DebugView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	vertex_buffer: gfx::Buffer<ColorVertex2D>,
	index_buffer: gfx::Buffer<u16>,

	active: bool,
}

impl DebugView {
	pub fn new(gfx: &gfx::Context) -> Result<DebugView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_2D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer::<ColorVertex2D>(gfx::BufferUsage::Stream);
		let mut index_buffer = gfx.new_buffer::<u16>(gfx::BufferUsage::Static);

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

			active: false,
		})
	}


	pub fn update(&mut self, debug_model: &crate::model::Debug) {
		self.active = debug_model.active;
		if !self.active {
			return
		}

		let vertices = [
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new(-0.02,-0.02), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new( 0.02,-0.02), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new( 0.02, 0.02), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new(-0.02, 0.02), Vec3::new(1.0, 1.0, 1.0)),
		];

		self.vertex_buffer.upload(&vertices);
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		if !self.active {
			return
		}

		let _section = ctx.perf.scoped_section("debug");

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.index_buffer.len());
	}
}