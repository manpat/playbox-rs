pub use toybox::prelude::*;
use crate::model;

pub struct BlobShadowView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	index_buffer: gfx::Buffer<u16>,
	instance_buffer: gfx::Buffer<Mat3x4>,
}

impl BlobShadowView {
	pub fn new(gfx: &gfx::Context) -> Result<BlobShadowView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_INSTANCED_TRANFORM_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let instance_buffer = gfx.new_buffer::<Mat3x4>();
		let mut vertex_buffer = gfx.new_buffer::<gfx::ColorVertex>();
		let mut index_buffer = gfx.new_buffer::<u16>();

		let vertices: Vec<_> = (0..36)
			.map(|idx| gfx::ColorVertex::new(Vec3::from_y_angle(idx as f32 / 36.0 * TAU) / 2.0, Vec3::zero()))
			.collect();

		let indices: Vec<u16> = (0..36)
			.flat_map(|idx| [0, idx, (idx+1) % 36])
			.collect();

		vertex_buffer.upload(&vertices, gfx::BufferUsage::Static);
		index_buffer.upload(&indices, gfx::BufferUsage::Static);

		let vao = gfx.new_vao();
		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		Ok(BlobShadowView {
			shader,
			vao,
			index_buffer,
			instance_buffer,
		})
	}

	pub fn update(&mut self, blob_shadow_model: &model::BlobShadowModel) {
		let instances: Vec<_> = blob_shadow_model.shadow_casters.iter()
			.map(|caster| {
				let pos = Vec3{y: 0.01, ..caster.position};
				let scale = caster.scale / (1.0 + caster.position.y).max(1.0);
				Mat3x4::scale_translate(Vec3::splat(scale), pos)
			})
			.collect();

		self.instance_buffer.upload(&instances, gfx::BufferUsage::Dynamic);
	}

	pub fn draw(&self, ctx: &mut super::ViewContext) {
		if self.instance_buffer.is_empty() {
			return
		}

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.bind_shader_storage_buffer(0, self.instance_buffer);
		ctx.gfx.draw_instances_indexed(gfx::DrawMode::Triangles, self.index_buffer.len(), self.instance_buffer.len());
	}
}