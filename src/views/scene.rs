use toybox::prelude::*;
use gfx::vertex::ColorVertex;
use gfx::mesh::{Mesh, MeshData};

use crate::model;

mod gem;

pub struct SceneView {
	shader: gfx::Shader,
	mesh: Mesh<ColorVertex>,

	gem_view: gem::GemView,
}

impl SceneView {
	pub fn new(gfx: &gfx::Context, scene: &model::Scene) -> Result<SceneView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let mut mesh_data = MeshData::new();
		let main_scene = scene.main_scene();

		for entity in main_scene.entities().filter(|e| !e.name.contains('_')) {
			build_entity_transformed(&mut mesh_data.vertices, &mut mesh_data.indices, entity, entity.transform());
		}

		let mut mesh = Mesh::new(gfx);
		mesh.upload(&mesh_data);

		Ok(SceneView {
			shader,
			mesh,

			gem_view: gem::GemView::new(gfx, scene)?,
		})
	}

	pub fn update(&mut self, scene: &model::Scene, blob_shadows: &mut model::BlobShadowModel) {
		self.gem_view.update(scene, blob_shadows);
	}

	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("scene");

		ctx.gfx.bind_shader(self.shader);
		self.mesh.draw(&ctx.gfx, gfx::DrawMode::Triangles);

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


