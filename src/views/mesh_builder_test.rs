use toybox::prelude::*;
use gfx::vertex::{ColorVertex, ColorVertex2D};
use gfx::mesh::*;

use crate::model;

pub struct MeshBuilderTestView {
	shader_3d: gfx::Shader,
	shader_2d: gfx::Shader,

	mesh_3d: Mesh<ColorVertex>,
	mesh_2d: Mesh<ColorVertex2D>,

	mesh_data_3d: MeshData<ColorVertex>,
	mesh_data_2d: MeshData<ColorVertex2D>,

	time: f32,
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

			time: 0.0,
		})
	}

	pub fn update(&mut self) {
		self.mesh_data_2d.clear();
		self.mesh_data_3d.clear();

		{
			let mut mb = ColorMeshBuilder::new(&mut self.mesh_data_3d);
			mb.set_color(Color::rgb(1.0, 0.5, 0.2));

			mb.build(geom::Tetrahedron::unit());
			mb.build(geom::Tetrahedron::from_matrix(Mat3x4::rotate_y_translate(PI*self.time, Vec3::from_y(2.0))));

			let plane = Mat3::from_columns([
				Vec3::from_x(1.0),
				Vec3::from_z(-1.0),
				Vec3::new(2.0, 0.05, 5.0),
			]);

			let mut pmb = mb.on_plane(plane);
			pmb.set_color(Color::rgb(0.5, 0.5, 0.9));
			pmb.build(geom::Polygon::unit(6));
			pmb.build(geom::Quad::from_matrix(Mat2x3::rotate_translate(
				TAU * self.time/5.0,
				3.0 * Vec2::from_angle(TAU * self.time / 4.0)
			)));

			let plane = Mat3::from_columns([
				Vec3::from_x(1.0),
				Vec3::from_z(-1.0),
				Vec3::new(2.0, 0.2, 5.0),
			]);

			let mut pmb = mb.on_plane(plane);
			pmb.set_color(Color::rgb(0.5, 0.8, 0.9));
			pmb.build(geom::Polygon::unit(5));
			pmb.build(geom::Quad::from_matrix(Mat2x3::rotate_translate(
				-TAU * self.time/5.0,
				3.0 * Vec2::from_angle(TAU * self.time / 4.0)
			)));
		}

		{
			let mut mb = ColorMeshBuilder2D::new(&mut self.mesh_data_2d);
			mb.set_color(Color::rgb(0.6, 1.0, 0.3));
			// mb.build(geom::Polygon::unit(7));
			mb.build(geom::Quad::from_matrix(Mat2x3::scale_translate(Vec2::splat(0.2), Vec2::new(1.0, 0.2))));
		}

		self.mesh_3d.upload(&self.mesh_data_3d);
		self.mesh_2d.upload(&self.mesh_data_2d);

		self.time += 1.0 / 60.0;
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