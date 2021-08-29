pub use toybox::prelude::*;
use gfx::mesh::Mesh;
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

		let vertices: Vec<_> = (0..36)
			.map(|idx| gfx::ColorVertex::new(Vec3::from_y_angle(idx as f32 / 36.0 * TAU) / 2.0, Vec3::zero()))
			.collect();

		let indices: Vec<u16> = (0..36)
			.flat_map(|idx| [0, idx, (idx+1) % 36])
			.collect();

		let instance_buffer = gfx.new_buffer::<Mat3x4>(gfx::BufferUsage::Stream);

		let mut mesh = Mesh::new(gfx);
		mesh.upload_separate(&vertices, &indices);

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
