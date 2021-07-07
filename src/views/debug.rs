use toybox::prelude::*;
use gfx::vertex::ColorVertex2D;

pub struct DebugView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	vertex_buffer: gfx::Buffer<ColorVertex2D>,
	num_elements: u32,

	active: bool,
}

impl DebugView {
	pub fn new(gfx: &gfx::Context) -> Result<DebugView, Box<dyn Error>> {
		let shader = gfx.new_shader(&[
			(gfx::raw::VERTEX_SHADER, include_str!("../shaders/color_2d.vert.glsl")),
			(gfx::raw::FRAGMENT_SHADER, include_str!("../shaders/flat_color.frag.glsl")),
		])?;

		let vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer::<ColorVertex2D>();
		let index_buffer = gfx.new_buffer::<u16>();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		let indices = [
			0, 1, 2,
			0, 2, 3,
		];

		index_buffer.upload(&indices, gfx::BufferUsage::Static);

		Ok(DebugView {
			shader,
			vao,
			vertex_buffer,
			num_elements: indices.len() as u32,

			active: false,
		})
	}


	pub fn update(&mut self, debug_model: &crate::model::Debug) {
		self.active = debug_model.active;
		if !self.active {
			return
		}

		let vertices = [
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new(-0.04,-0.04), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new( 0.04,-0.04), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new( 0.04, 0.04), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex2D::new(debug_model.mouse_pos + Vec2::new(-0.04, 0.04), Vec3::new(1.0, 1.0, 1.0)),
		];

		self.vertex_buffer.upload(&vertices, gfx::BufferUsage::Dynamic);
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		if !self.active {
			return
		}

		let _section = ctx.perf.scoped_section("debug");

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.num_elements);
	}
}