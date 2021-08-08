use toybox::prelude::*;
use gfx::vertex::ColorVertex;

use crate::model::{Player, BlobShadowModel};
use crate::mesh::Mesh;

pub struct PlayerView {
	shader: gfx::Shader,
	mesh: Mesh<ColorVertex>,

	player_hat_pos: Vec3,
	player_vel: Vec3,
}

impl PlayerView {
	pub fn new(gfx: &gfx::Context) -> Result<PlayerView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let mesh = Mesh::new(gfx);

		Ok(PlayerView {
			shader,
			mesh,

			player_hat_pos: Vec3::new(0.0, 2.0, 0.0),
			player_vel: Vec3::zero(),
		})
	}


	pub fn update(&mut self, player: &Player, blob_shadows: &mut BlobShadowModel) {
		let body_transform = Mat3x4::rotate_y_translate(player.yaw, player.body_position);

		let foot_size = 0.3;
		let left_foot_color = Vec3::new(1.0, 0.8, 0.5);
		let right_foot_color = Vec3::new(0.5, 0.8, 1.0);

		let vertices = [
			ColorVertex::new(body_transform * Vec3::new(-1.0, 0.0,-1.0), Vec3::new(0.5, 0.5, 1.0)),
			ColorVertex::new(body_transform * Vec3::new( 1.0, 0.0,-1.0), Vec3::new(0.5, 0.5, 1.0)),
			ColorVertex::new(body_transform * Vec3::new( 1.0, 0.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
			ColorVertex::new(body_transform * Vec3::new(-1.0, 0.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),

			ColorVertex::new(self.player_hat_pos, Vec3::new(1.0, 0.0, 1.0)),

			ColorVertex::new(player.feet_positions[0] + Vec3::new(-foot_size, 0.1,-foot_size), left_foot_color),
			ColorVertex::new(player.feet_positions[0] + Vec3::new( foot_size, 0.1,-foot_size), left_foot_color),
			ColorVertex::new(player.feet_positions[0] + Vec3::new( foot_size, 0.1, foot_size), left_foot_color),
			ColorVertex::new(player.feet_positions[0] + Vec3::new(-foot_size, 0.1, foot_size), left_foot_color),

			ColorVertex::new(player.feet_positions[1] + Vec3::new(-foot_size, 0.1,-foot_size), right_foot_color),
			ColorVertex::new(player.feet_positions[1] + Vec3::new( foot_size, 0.1,-foot_size), right_foot_color),
			ColorVertex::new(player.feet_positions[1] + Vec3::new( foot_size, 0.1, foot_size), right_foot_color),
			ColorVertex::new(player.feet_positions[1] + Vec3::new(-foot_size, 0.1, foot_size), right_foot_color),
		];

		let indices = [
			// Bottom
			0, 1, 2,
			0, 2, 3,

			// Body
			4, 0, 1,
			4, 1, 2,
			4, 2, 3,
			4, 3, 0,

			// Feet
			5, 6, 7,
			5, 7, 8,

			9, 10, 11,
			9, 11, 12,
		];

		self.mesh.upload_separate(&vertices, &indices);

		// TODO(pat.m): move to model and controller
		let position_diff = player.body_position - self.player_hat_pos + Vec3::from_y(2.0);
		self.player_vel *= 0.92;
		self.player_vel += position_diff * 0.05;
		self.player_hat_pos += self.player_vel;

		blob_shadows.add(player.body_position, 2.0);
	}


	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("player");

		ctx.gfx.bind_shader(self.shader);
		self.mesh.draw(&ctx.gfx, gfx::DrawMode::Triangles);
	}
}