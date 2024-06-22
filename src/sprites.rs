use toybox::prelude::*;


#[derive(Debug)]
pub struct Sprites {
	vertices: Vec<gfx::StandardVertex>,
	indices: Vec<u32>,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,

	atlas: gfx::ImageHandle,

	up: Vec3,
	right: Vec3,
}

impl Sprites {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<Sprites> {
		Ok(Sprites {
			vertices: Vec::new(),
			indices: Vec::new(),

			v_shader: gfx.resource_manager.standard_vs_shader,
			f_shader: gfx.resource_manager.flat_fs_shader,

			atlas: gfx.resource_manager.request(gfx::LoadImageRequest::from("images/coolcat.png")),

			up: Vec3::from_y(1.0),
			right: Vec3::from_x(1.0),
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

	pub fn set_billboard_orientation(&mut self, up: Vec3, right: Vec3) {
		self.up = up;
		self.right = right;
	}
}

impl Sprites {
	pub fn add(&mut self, right: Vec3, up: Vec3, pos: Vec3, color: impl Into<Color>) {
		let start_index = self.vertices.len() as u32;
		let indices = [0, 1, 2, 0, 2, 3].into_iter().map(|i| i + start_index);

		let right = right/2.0;
		let color = color.into();

		let vertices = [
			gfx::StandardVertex::new(pos - right, Vec2::new(0.0, 0.0), color),
			gfx::StandardVertex::new(pos - right + up, Vec2::new(0.0, 1.0), color),
			gfx::StandardVertex::new(pos + right + up, Vec2::new(1.0, 1.0), color),
			gfx::StandardVertex::new(pos + right, Vec2::new(1.0, 0.0), color),
		];

		self.vertices.extend_from_slice(&vertices);
		self.indices.extend(indices);
	}

	pub fn billboard(&mut self, pos: Vec3, size: Vec2, color: impl Into<Color>) {
		self.add(size.x * self.right, size.y * self.up, pos, color);
	}
}
