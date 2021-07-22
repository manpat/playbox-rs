use toybox::prelude::*;
use gfx::vertex::ColorVertex;

use crate::model;

mod gem;

pub struct SceneView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	index_buffer: gfx::Buffer<u16>,

	gem_view: gem::GemView,
}

impl SceneView {
	pub fn new(gfx: &gfx::Context, scene: &model::Scene) -> Result<SceneView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let vao = gfx.new_vao();

		let mut vertex_buffer = gfx.new_buffer::<ColorVertex>();
		let mut index_buffer = gfx.new_buffer::<u16>();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		let mut vertices = Vec::new();
		let mut indices = Vec::new();

		let main_scene = scene.source_data.find_scene("main").unwrap();

		for entity in main_scene.entities().filter(|e| !e.name.contains('_')) {
			build_entity_transformed(&mut vertices, &mut indices, entity, entity.transform());
		}

		vertex_buffer.upload(&vertices, gfx::BufferUsage::Static);
		index_buffer.upload(&indices, gfx::BufferUsage::Static);

		Ok(SceneView {
			shader,
			vao,
			index_buffer,

			gem_view: gem::GemView::new(gfx, scene)?,
		})
	}

	pub fn update(&mut self, scene: &model::Scene, blob_shadows: &mut model::BlobShadowModel) {
		self.gem_view.update(scene, blob_shadows);
	}

	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("scene");

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.index_buffer.len());

		self.gem_view.draw(&ctx.gfx);
	}
}



fn build_entity_transformed(vertices: &mut Vec<ColorVertex>, indices: &mut Vec<u16>,
	entity: toy::EntityRef<'_>, transform: Mat3x4)
{
	let mesh_data = entity.mesh_data().unwrap();

	let color_data = mesh_data.color_data(None).unwrap();

	let ent_vertices = mesh_data.positions.iter()
		.zip(&color_data.data)
		.map(move |(&p, &col)| {
			let p = transform * p;
			ColorVertex::new(p, col.to_vec3())
		});

	let vertex_base = vertices.len() as u16;

	vertices.extend(ent_vertices);
	indices.extend(mesh_data.indices.iter().map(|&i| vertex_base + i));
}


