use toybox::prelude::*;
use gl::vertex::ColorVertex;

use crate::model::Player;

pub struct PlayerView {
	shader: gl::Shader,
	vao: gl::Vao,
	vertex_buffer: gl::Buffer<ColorVertex>,
	index_buffer: gl::Buffer<u16>,
	num_elements: u32,

	player_hat_pos: Vec3,
	player_vel: Vec3,
}

impl PlayerView {
	pub fn new(gl: &gl::Context) -> Result<PlayerView, Box<dyn Error>> {
		let shader = gl.new_shader(&[
			(gl::raw::VERTEX_SHADER, include_str!("../shaders/color_3d.vert.glsl")),
			(gl::raw::FRAGMENT_SHADER, include_str!("../shaders/flat_color.frag.glsl")),
		])?;

		let vao = gl.new_vao();

		let vertex_buffer = gl.new_buffer();
		let index_buffer = gl.new_buffer();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		Ok(PlayerView {
			shader,
			vao,
			vertex_buffer,
			index_buffer,
			num_elements: 0,

			player_hat_pos: Vec3::new(0.0, 1.0, 0.0),
			player_vel: Vec3::zero(),
		})
	}


	pub fn update(&mut self, player: &Player) {
		let body_transform = Mat3x4::rotate_y_translate(player.yaw, player.position);

		let vertices = [
			ColorVertex::new(body_transform * Vec3::new(-1.0,-1.0,-1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(body_transform * Vec3::new( 1.0,-1.0,-1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(body_transform * Vec3::new( 1.0,-1.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(body_transform * Vec3::new(-1.0,-1.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),

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

		self.vertex_buffer.upload(&vertices, gl::BufferUsage::Dynamic);
		self.index_buffer.upload(&indices, gl::BufferUsage::Dynamic);

		self.num_elements = indices.len() as u32;

		let position_diff = player.position.to_xz() - self.player_hat_pos.to_xz();
		self.player_vel *= 0.92;
		self.player_vel += position_diff.to_x0z() * 0.05;
		self.player_hat_pos += self.player_vel;
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("player");

		ctx.gl.bind_vao(self.vao);
		ctx.gl.bind_shader(self.shader);
		ctx.gl.draw_indexed(gl::DrawMode::Triangles, self.num_elements);
	}
}