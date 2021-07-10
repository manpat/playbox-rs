use toybox::prelude::*;
use gfx::vertex::ColorVertex;

pub struct SceneView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	num_elements: u32,
}

impl SceneView {
	pub fn new(gfx: &gfx::Context, project: &toy::Project) -> Result<SceneView, Box<dyn Error>> {
		let shader = gfx.new_shader(&[
			(gfx::raw::VERTEX_SHADER, include_str!("../shaders/color_3d.vert.glsl")),
			(gfx::raw::FRAGMENT_SHADER, include_str!("../shaders/flat_color.frag.glsl")),
		])?;

		let vao = gfx.new_vao();

		let mut vertex_buffer = gfx.new_buffer::<ColorVertex>();
		let mut index_buffer = gfx.new_buffer::<u16>();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		let mut vertices = Vec::new();
		let mut indices = Vec::new();

		let scene = project.find_scene("main").unwrap();

		for entity in scene.entities() {
			let mesh_data = entity.mesh_data().unwrap();
			let transform = entity.transform();

			let color_data = mesh_data.color_data(None).unwrap();

			let ent_vertices = mesh_data.positions.iter()
				.zip(&color_data.data)
				.map(|(&p, &col)| {
					let p = transform * p;
					ColorVertex::new(p, col.to_vec3())
				});

			let vertex_base = vertices.len() as u16;

			vertices.extend(ent_vertices);
			indices.extend(mesh_data.indices.iter().map(|&i| vertex_base + i));
		}

		vertex_buffer.upload(&vertices, gfx::BufferUsage::Static);
		index_buffer.upload(&indices, gfx::BufferUsage::Static);

		Ok(SceneView {
			shader,
			vao,
			num_elements: indices.len() as u32,
		})
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("scene");

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.num_elements);
	}
}