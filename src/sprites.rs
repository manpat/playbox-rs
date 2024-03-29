use toybox::prelude::*;


#[derive(Debug)]
pub struct Sprites {
	vertices: Vec<gfx::StandardVertex>,
	indices: Vec<u32>,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,

	atlas: gfx::ImageHandle,
}

impl Sprites {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<Sprites> {
		Ok(Sprites {
			vertices: Vec::new(),
			indices: Vec::new(),

			v_shader: gfx.resource_manager.standard_vs_shader,
			f_shader: gfx.resource_manager.flat_fs_shader,

			atlas: gfx.resource_manager.request(gfx::LoadImageRequest::from("images/coolcat.png")),
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System) {
		if self.vertices.is_empty() {
			return
		}

		gfx.frame_encoder.command_group(gfx::FrameStage::Main)
			.annotate("Sprites")
			.draw(self.v_shader, self.f_shader)
			.elements(self.indices.len() as u32)
			.indexed(&self.indices)
			.ssbo(0, &self.vertices)
			.sampled_image(0, self.atlas, gfx.resource_manager.nearest_sampler);

		self.vertices.clear();
		self.indices.clear();
	}
}

impl Sprites {
	pub fn basic(&mut self, right: Vec3, up: Vec3, pos: Vec3, color: Color) {
		let start_index = self.vertices.len() as u32;
		let indices = [0, 1, 2, 0, 2, 3].into_iter().map(|i| i + start_index);

		let right = right/2.0;

		let vertices = [
			gfx::StandardVertex::new(pos - right, Vec2::new(0.0, 0.0), color),
			gfx::StandardVertex::new(pos - right + up, Vec2::new(0.0, 1.0), color),
			gfx::StandardVertex::new(pos + right + up, Vec2::new(1.0, 1.0), color),
			gfx::StandardVertex::new(pos + right, Vec2::new(1.0, 0.0), color),
		];

		self.vertices.extend_from_slice(&vertices);
		self.indices.extend(indices);
	}
}
