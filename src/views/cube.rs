use toybox::prelude::*;
use gl::vertex::ColorVertex;

pub struct CubeView {
	shader: gl::Shader,
	vao: gl::Vao,
	num_elements: u32,
}

impl CubeView {
	pub fn new(gl: &gl::Context) -> Result<CubeView, Box<dyn Error>> {
		let shader = gl.new_shader(&[
			(gl::raw::VERTEX_SHADER, include_str!("../shaders/color_3d.vert.glsl")),
			(gl::raw::FRAGMENT_SHADER, include_str!("../shaders/flat_color.frag.glsl")),
		])?;

		let vao = gl.new_vao();

		let vertex_buffer = gl.new_buffer::<ColorVertex>();
		let index_buffer = gl.new_buffer::<u16>();

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

		vertex_buffer.upload(&vertices, gl::BufferUsage::Static);
		index_buffer.upload(&indices, gl::BufferUsage::Static);

		Ok(CubeView {
			shader,
			vao,
			num_elements: indices.len() as u32,
		})
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		ctx.gl.bind_vao(self.vao);
		ctx.gl.bind_shader(self.shader);

		ctx.perf.start_section("cube");
		ctx.gl.draw_indexed(gl::DrawMode::Triangles, self.num_elements);
		ctx.perf.end_section();
	}
}