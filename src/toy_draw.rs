use toybox::prelude::*;


#[derive(Debug)]
pub struct ToyRenderer {
	texture: gfx::ImageNameOrHandle,
	color_target: Option<gfx::ImageHandle>,
	depth_target: Option<gfx::ImageHandle>,
	framestage: gfx::FrameStage,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,

	vertex_buffer: gfx::BufferName,
	index_buffer: gfx::BufferName,
	element_count: u32,

	builder: ToyMeshBuilder,
}

impl ToyRenderer {
	pub fn new(core: &gfx::Core, rm: &mut gfx::ResourceManager) -> ToyRenderer {
		ToyRenderer {
			texture: rm.blank_white_image.into(),
			color_target: None,
			depth_target: None,
			framestage: gfx::FrameStage::Main,

			v_shader: rm.standard_vs_shader,
			f_shader: rm.flat_fs_shader,

			vertex_buffer: core.create_buffer(),
			index_buffer: core.create_buffer(),
			element_count: 0,

			builder: ToyMeshBuilder::new(),
		}
	}

	pub fn set_color_target(&mut self, target: impl Into<Option<gfx::ImageHandle>>) {
		self.color_target = target.into();
	}

	pub fn set_depth_target(&mut self, target: impl Into<Option<gfx::ImageHandle>>) {
		self.depth_target = target.into();
	}

	pub fn update(&mut self, core: &gfx::Core, f: impl FnOnce(&mut ToyMeshBuilder)) {
		self.builder.clear();
		self.builder.set_root_transform(Mat3x4::identity());
		f(&mut self.builder);

		self.element_count = self.builder.indices.len() as u32;
		core.upload_immutable_buffer_immediate(self.vertex_buffer, &self.builder.vertices);
		core.upload_immutable_buffer_immediate(self.index_buffer, &self.builder.indices);
	}

	pub fn draw(&self, gfx: &mut gfx::System) {
		if self.element_count > 0 {
			let mut group = gfx.frame_encoder.command_group(self.framestage);
			let mut command = group.draw(self.v_shader, self.f_shader);

			command.indexed(self.index_buffer)
				.ssbo(0, self.vertex_buffer)
				.sampled_image(0, self.texture, gfx.resource_manager.nearest_sampler)
				.elements(self.element_count);

			match (self.color_target, self.depth_target) {
				(Some(ct), Some(dt)) => { command.rendertargets(&[ct, dt]); }
				(Some(ct), None) => { command.rendertargets(&[ct]); }
				(None, Some(dt)) => { command.rendertargets(&[dt]); }
				_ => {}
			}
		}
	}
}


#[derive(Debug, Default)]
pub struct ToyMeshBuilder {
	vertices: Vec<gfx::StandardVertex>,
	indices: Vec<u32>,

	root_transform: Mat3x4,
}

impl ToyMeshBuilder {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn clear(&mut self) {
		self.vertices.clear();
		self.indices.clear();
	}

	pub fn set_root_transform(&mut self, transform: Mat3x4) {
		self.root_transform = transform;
	}

	pub fn add_mesh(&mut self, mesh: &toy::Mesh, transform: Mat3x4) {
		let transform = self.root_transform * transform;

		let index_start = self.vertices.len() as u32;
		let indices = mesh.indices.iter().map(|idx| *idx as u32 + index_start);

		let vertices = ToyMeshStandardVertexIterator::new(mesh)
			.map(|(pos, color, uv)| gfx::StandardVertex::new(transform * pos, uv, color));

		self.vertices.extend(vertices);
		self.indices.extend(indices);
	}

	pub fn add_entity(&mut self, entity: toy::EntityRef<'_>) {
		if let Some(mesh) = entity.mesh() {
			self.add_mesh(mesh, entity.transform());
		}
	}

	pub fn add_entities<'t>(&mut self, container: impl toy::EntityCollection<'t>) {
		for entity in container.into_entities() {
			if let Some(mesh) = entity.mesh() {
				self.add_mesh(mesh, entity.transform());
			}
		}
	}

	pub fn add_entities_with_prefix<'t>(&mut self, container: impl toy::EntityCollection<'t>, prefix: &str) {
		for entity in container.into_entities_with_prefix(prefix) {
			if let Some(mesh) = entity.mesh() {
				self.add_mesh(mesh, entity.transform());
			}
		}
	}
}

struct ToyMeshStandardVertexIterator<'t> {
	positions: &'t [Vec3],
	colors: Option<&'t [Vec4]>,
	uvs: Option<&'t [Vec2]>,
}

impl<'t> ToyMeshStandardVertexIterator<'t> {
	pub fn new(mesh: &'t toy::Mesh) -> Self {
		let positions = &mesh.positions;

		ToyMeshStandardVertexIterator {
			positions,
			colors: mesh.color_layers.first()
				.map(|layer| layer.data.as_slice())
				.filter(|data| data.len() == positions.len()),

			uvs: mesh.uv_layers.first()
				.map(|layer| layer.data.as_slice())
				.filter(|data| data.len() == positions.len()),
		}
	}
}

impl<'t> Iterator for ToyMeshStandardVertexIterator<'t> {
	type Item = (Vec3, Vec4, Vec2);

	fn next(&mut self) -> Option<Self::Item> {
		fn split_first<T: Copy>(s: &mut &[T]) -> Option<T> {
			let value = *s.first()?;
			*s = &s[1..];
			Some(value)
		}

		let pos = split_first(&mut self.positions)?;
		let color = self.colors.as_mut().and_then(split_first).unwrap_or_else(Vec4::one);
		let uv = self.uvs.as_mut().and_then(split_first).unwrap_or_else(Vec2::zero);

		Some((pos, color, uv))
	}
}