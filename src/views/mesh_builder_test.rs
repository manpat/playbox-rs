use toybox::prelude::*;
use gfx::vertex::{ColorVertex, ColorVertex2D};

use crate::model;
use crate::mesh::{self, geom, Mesh, MeshData};

pub struct MeshBuilderTestView {
	shader_3d: gfx::Shader,
	shader_2d: gfx::Shader,

	mesh_3d: Mesh<ColorVertex>,
	mesh_2d: Mesh<ColorVertex2D>,

	mesh_data_3d: MeshData<ColorVertex>,
	mesh_data_2d: MeshData<ColorVertex2D>,
}

impl MeshBuilderTestView {
	pub fn new(gfx: &gfx::Context) -> Result<Self, Box<dyn Error>> {
		let shader_3d = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let shader_2d = gfx.new_simple_shader(
			crate::shaders::COLOR_2D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let mesh_3d = Mesh::new(gfx);
		let mesh_2d = Mesh::new(gfx);

		let mesh_data_3d = MeshData::new();
		let mesh_data_2d = MeshData::new();

		Ok(MeshBuilderTestView {
			shader_3d,
			shader_2d,
			mesh_3d,
			mesh_2d,
			mesh_data_3d,
			mesh_data_2d,
		})
	}

	pub fn update(&mut self) {
		{
			let mut mb = mesh::ColorMeshBuilder::new(&mut self.mesh_data_3d);
			mb.set_color(Color::rgb(1.0, 0.5, 0.2));

			geom::Tetrahedron::unit().build(&mut mb);
			geom::Tetrahedron::from_matrix(Mat3x4::rotate_y_translate(PI, Vec3::from_y(2.0)))
				.build(&mut mb);
		}

		// {
		// 	let mut mb = mesh::ColorMeshBuilder2D::new(&mut self.mesh_data_2d);
		// }

		self.mesh_3d.upload(&self.mesh_data_3d);
		self.mesh_2d.upload(&self.mesh_data_2d);
	}

	pub fn draw(&self, ctx: &mut super::ViewContext) {
		ctx.gfx.bind_shader(self.shader_3d);
		self.mesh_3d.draw(&ctx.gfx, gfx::DrawMode::Triangles);
	}

	pub fn draw_2d(&self, ctx: &mut super::ViewContext) {
		ctx.gfx.bind_shader(self.shader_2d);
		self.mesh_2d.draw(&ctx.gfx, gfx::DrawMode::Triangles);
	}
}