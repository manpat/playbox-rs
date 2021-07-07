use toybox::prelude::*;
use gfx::vertex::ColorVertex;

pub struct CubeView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	num_elements: u32,
}

impl CubeView {
	pub fn new(gfx: &gfx::Context) -> Result<CubeView, Box<dyn Error>> {
		let shader = gfx.new_shader(&[
			(gfx::raw::VERTEX_SHADER, include_str!("../shaders/color_3d.vert.glsl")),
			(gfx::raw::FRAGMENT_SHADER, include_str!("../shaders/flat_color.frag.glsl")),
		])?;

		let vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer::<ColorVertex>();
		let index_buffer = gfx.new_buffer::<u16>();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		let vertices = [
			ColorVertex::new(Vec3::new(-1.0,-1.0,-1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(Vec3::new( 1.0,-1.0,-1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(Vec3::new( 1.0,-1.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(Vec3::new(-1.0,-1.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),

			ColorVertex::new(Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 1.0)),
		];

		let indices = [
			0, 1, 2,
			0, 2, 3,

			4, 0, 1,
			4, 1, 2,
			4, 2, 3,
			4, 3, 0,
		];

		vertex_buffer.upload(&vertices, gfx::BufferUsage::Static);
		index_buffer.upload(&indices, gfx::BufferUsage::Static);

		Ok(CubeView {
			shader,
			vao,
			num_elements: indices.len() as u32,
		})
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("cube");

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.num_elements);
	}
}