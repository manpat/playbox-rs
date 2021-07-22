use toybox::prelude::*;
use toybox::perf::Instrumenter;
use gfx::vertex::ColorVertex2D;

pub struct PerfView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	vertex_buffer: gfx::Buffer<ColorVertex2D>,
	index_buffer: gfx::Buffer<u16>,
}

impl PerfView {
	pub fn new(gfx: &gfx::Context) -> Result<PerfView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_2D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer();
		let index_buffer = gfx.new_buffer();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		Ok(PerfView {
			shader,
			vao,
			vertex_buffer,
			index_buffer,
		})
	}


	pub fn update(&mut self, inst: &Instrumenter, aspect: f32) {
		let summary = match inst.summary() {
			Some(summary) => summary,
			None => return
		};

		let size = 0.2;

		let mut builder = Builder2D {
			vertices: Vec::new(),
			indices: Vec::new(),

			transform: Mat2x3::scale_translate(Vec2::splat(size), Vec2::new(size - aspect, size - 1.0)),
		};


		let total_angle = (2.0 * PI) / summary.total_cpu_time_ms as f32;
		let mut current_angle = 0.0;

		for (idx, section) in summary.sections.iter().enumerate() {
			let section_angle = section.cpu_time_ms as f32 * total_angle;

			let color = Color::hsv((idx as f32 * 40.0) % 360.0, 0.7, 0.7);

			builder.build_wedge(current_angle, current_angle + section_angle, color);

			current_angle += section_angle;
		}


		builder.transform = Mat2x3::scale_translate(Vec2::splat(size), Vec2::new(size - aspect + 2.1 * size, size - 1.0));

		let total_angle = (2.0 * PI) / summary.total_gpu_time_ms as f32;
		let mut current_angle = 0.0;

		for (idx, section) in summary.sections.iter().enumerate() {
			let section_angle = section.gpu_time_ms as f32 * total_angle;

			let color = Color::hsv((idx as f32 * 40.0) % 360.0, 0.7, 0.7);

			builder.build_wedge(current_angle, current_angle + section_angle, color);

			current_angle += section_angle;
		}


		builder.transform = Mat2x3::scale_translate(Vec2::splat(size), Vec2::new(size - aspect + 4.2 * size, size - 1.0));

		let total_angle = (2.0 * PI) / summary.total_triangles as f32;
		let mut current_angle = 0.0;

		for (idx, section) in summary.sections.iter().enumerate() {
			let section_angle = section.triangles as f32 * total_angle;

			let color = Color::hsv((idx as f32 * 40.0) % 360.0, 0.7, 0.7);

			builder.build_wedge(current_angle, current_angle + section_angle, color);

			current_angle += section_angle;
		}


		self.vertex_buffer.upload(&builder.vertices, gfx::BufferUsage::Static);
		self.index_buffer.upload(&builder.indices, gfx::BufferUsage::Static);
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("perf");

		if self.index_buffer.is_empty() {
			return
		}

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);

		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.index_buffer.len());
	}
}



struct Builder2D {
	vertices: Vec<ColorVertex2D>,
	indices: Vec<u16>,

	transform: Mat2x3,
}

impl Builder2D {
	fn build_wedge(&mut self, mut start_angle: f32, mut end_angle: f32, color: Color) {
		if start_angle > end_angle {
			std::mem::swap(&mut start_angle, &mut end_angle);
		}

		let color = color.to_vec3();
		let angle_diff = end_angle - start_angle;
		let vert_angle_threshold = PI / 36.0;

		let num_triangles = (angle_diff / vert_angle_threshold) as u32;
		let num_triangles = num_triangles.max(1);

		let inc = angle_diff / num_triangles as f32;

		let index_start = self.vertices.len() as u32;

		self.vertices.push(ColorVertex2D::new(self.transform.column_z(), color));

		for vertex_idx in 0..=num_triangles {
			let angle = vertex_idx as f32 * inc + start_angle;
			let offset = self.transform * Vec2::from_angle(angle);
			self.vertices.push(ColorVertex2D::new(offset, color));
		}

		for triangle in 0..num_triangles {
			self.indices.push(index_start as u16);
			self.indices.push((index_start + triangle + 1) as u16);
			self.indices.push((index_start + triangle + 2) as u16);
		}
	}
}