use toybox::prelude::*;
use gfx::vertex::ColorVertex2D;
use crate::model;

pub struct SrgbView {
	shader: gfx::Shader,
	gradient_mesh: gfx::Mesh<ColorVertex2D>,

	gradient_positions: Vec<Vec2>,
	gradient_colors: Vec<Color>,
	gradient_indices: Vec<u16>,

	time: f32,
}

impl SrgbView {
	pub fn new(gfx: &mut gfx::Context, scene: &model::Scene) -> Result<SrgbView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_2D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let gradient_mesh_src = scene.source_data.find_entity("SRGB_gradients")
			.expect("Failed to find SRGB_gradients entity")
			.mesh_data()
			.expect("SRGB_gradients missing mesh data");

		let color_data = gradient_mesh_src.color_data(None)
			.expect("Missing color data");

		let gradient_positions = gradient_mesh_src.positions.iter().map(Vec3::to_xy).collect();
		let gradient_colors = color_data.data.iter().copied().map(Color::from).collect();
		let gradient_indices = gradient_mesh_src.indices.clone();

		let gradient_mesh = gfx::Mesh::new(gfx);

		Ok(SrgbView {
			shader,
			gradient_mesh,

			gradient_positions,
			gradient_colors,
			gradient_indices,

			time: 0.0,
		})
	}

	pub fn update(&mut self) {
		let vertices = self.gradient_positions.iter().cloned()
			.zip(&self.gradient_colors)
			.map(|(p, color)| ColorVertex2D::new(p, *color));

		let mut mesh_data = gfx::MeshData::new();
		mesh_data.extend(vertices, self.gradient_indices.iter().cloned());
		self.gradient_mesh.upload(&mesh_data);
	}

	pub fn draw(&self, ctx: &mut crate::views::ViewContext) {
		ctx.gfx.bind_shader(self.shader);
		self.gradient_mesh.draw(&mut ctx.gfx, gfx::DrawMode::Triangles);
	}
}