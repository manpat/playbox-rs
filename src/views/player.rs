use toybox::prelude::*;
use gfx::vertex::ColorVertex;

use crate::model::Player;

pub struct PlayerView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	vertex_buffer: gfx::Buffer<ColorVertex>,
	index_buffer: gfx::Buffer<u16>,
	num_elements: u32,

	player_hat_pos: Vec3,
	player_vel: Vec3,
}

impl PlayerView {
	pub fn new(gfx: &gfx::Context) -> Result<PlayerView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer();
		let index_buffer = gfx.new_buffer();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		Ok(PlayerView {
			shader,
			vao,
			vertex_buffer,
			index_buffer,
			num_elements: 0,

			player_hat_pos: Vec3::new(0.0, 2.0, 0.0),
			player_vel: Vec3::zero(),
		})
	}


	pub fn update(&mut self, player: &Player) {
		let body_transform = Mat3x4::rotate_y_translate(player.yaw, player.position);

		let vertices = [
			ColorVertex::new(body_transform * Vec3::new(-1.0, 0.0,-1.0), Vec3::new(1.0, 0.5, 1.0)),
			ColorVertex::new(body_transform * Vec3::new( 1.0, 0.0,-1.0), Vec3::new(1.0, 0.5, 1.0)),
			ColorVertex::new(body_transform * Vec3::new( 1.0, 0.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(body_transform * Vec3::new(-1.0, 0.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),

			ColorVertex::new(self.player_hat_pos, Vec3::new(1.0, 0.0, 1.0)),
		];

		let indices = [
			0, 1, 2,
			0, 2, 3,

			4, 0, 1,
			4, 1, 2,
			4, 2, 3,
			4, 3, 0,
		];

		self.vertex_buffer.upload(&vertices, gfx::BufferUsage::Dynamic);
		self.index_buffer.upload(&indices, gfx::BufferUsage::Dynamic);

		self.num_elements = indices.len() as u32;

		let position_diff = player.position.to_xz() - self.player_hat_pos.to_xz();
		self.player_vel *= 0.92;
		self.player_vel += position_diff.to_x0z() * 0.05;
		self.player_hat_pos += self.player_vel;
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("player");

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.num_elements);
	}
}