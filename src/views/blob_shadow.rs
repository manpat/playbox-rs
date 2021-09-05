pub use toybox::prelude::*;
use gfx::mesh::*;
use crate::model;

pub struct BlobShadowView {
	shader: gfx::Shader,
	mesh: Mesh<gfx::ColorVertex>,
	instance_buffer: gfx::Buffer<Mat3x4>,
}

impl BlobShadowView {
	pub fn new(gfx: &gfx::Context) -> Result<BlobShadowView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_INSTANCED_TRANFORM_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let instance_buffer = gfx.new_buffer::<Mat3x4>(gfx::BufferUsage::Stream);

		let plane = Mat3::from_columns([
			Vec3::from_x(1.0),
			Vec3::from_z(-1.0),
			Vec3::zero(),
		]);

		let mut mesh_data = MeshData::new();
		let mut mb = ColorMeshBuilder::new(&mut mesh_data).on_plane(plane);

		mb.set_color(Vec3::zero());
		mb.build(geom::Polygon::unit(36));

		let mesh = Mesh::from_mesh_data(gfx, &mesh_data);

		Ok(BlobShadowView {
			shader,
			mesh,
			instance_buffer,
		})
	}

	pub fn update(&mut self, blob_shadow_model: &model::BlobShadowModel, scene: &model::Scene) {
		use crate::intersect::{Ray, scene_raycast};

		let scene = scene.main_scene();

		let instances: Vec<_> = blob_shadow_model.shadow_casters.iter()
			.filter_map(|caster| {
				let ray = Ray { position: caster.position, direction: Vec3::from_y(-1.0) };
				let result = scene_raycast(&scene, &ray)?;
				Some((caster, result))
			})
			.map(|(caster, raycast_pos)| {
				let pos = raycast_pos + Vec3::from_y(0.01);
				let scale = caster.scale / (1.0 + caster.position.y - raycast_pos.y).max(1.0);
				Mat3x4::scale_translate(Vec3::splat(scale), pos)
			})
			.collect();

		self.instance_buffer.upload(&instances);
	}

	pub fn draw(&self, ctx: &mut super::ViewContext) {
		if self.instance_buffer.is_empty() {
			return
		}

		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.bind_shader_storage_buffer(0, self.instance_buffer);
		self.mesh.draw_instanced(&ctx.gfx, gfx::DrawMode::Triangles, self.instance_buffer.len());
	}
}
