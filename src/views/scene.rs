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
	pub fn new(gfx: &mut gfx::Context, scene: &model::Scene) -> Result<SceneView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let mut mesh_data = MeshData::new();
		let main_scene = scene.main_scene();

		for entity in main_scene.entities().filter(|e| !e.name.contains('_') || e.name.starts_with("SOUND_")) {
			build_entity_transformed(&mut mesh_data, entity, entity.transform());
		}

		let mut mesh = Mesh::new(gfx);
		mesh.upload(&mesh_data);

		Ok(SceneView {
			shader,
			mesh,

			gem_view: gem::GemView::new(gfx, scene)?,
		})
	}

	#[instrument(skip_all)]
	pub fn update(&mut self, scene: &model::Scene, blob_shadows: &mut model::BlobShadowModel) {
		self.gem_view.update(scene, blob_shadows);
	}

	#[instrument(skip_all)]
	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("scene");

		ctx.gfx.bind_shader(self.shader);
		self.mesh.draw(&mut ctx.gfx, gfx::DrawMode::Triangles);

		self.gem_view.draw(&mut ctx.gfx);
	}
}



fn build_entity_transformed(mesh_data: &mut MeshData<ColorVertex>,
	entity: toy::EntityRef<'_>, transform: Mat3x4)
{
	let ent_mesh_data = entity.mesh_data().unwrap();
	let color_data = ent_mesh_data.color_data(None).unwrap();

	let ent_vertices = ent_mesh_data.positions.iter()
		.zip(&color_data.data)
		.map(move |(&p, &col)| {
			let p = transform * p;
			ColorVertex::new(p, col)
		});

	mesh_data.extend(ent_vertices, ent_mesh_data.indices.iter().cloned());
}
