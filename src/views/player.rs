use toybox::prelude::*;
use gfx::vertex::ColorVertex;
use gfx::mesh::*;

use crate::model::Player;

pub struct PlayerView {
	shader: gfx::Shader,
	mesh: Mesh<ColorVertex>,
	mesh_data: MeshData<ColorVertex>,

	player_hat_pos: Vec3,
	player_vel: Vec3,

	blink_timer: f32,
}

impl PlayerView {
	pub fn new(gfx: &mut gfx::Context) -> Result<PlayerView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let mesh = Mesh::new(gfx);

		Ok(PlayerView {
			shader,
			mesh,
			mesh_data: MeshData::new(),

			player_hat_pos: Vec3::new(0.0, 2.0, 0.0),
			player_vel: Vec3::zero(),

			blink_timer: 0.0,
		})
	}

	#[instrument(skip_all)]
	pub fn update(&mut self, player: &Player) {
		let body_transform = Mat3x4::rotate_y_translate(player.yaw, player.body_position);

		let foot_size = 0.7;
		let body_color = Color::rgb(1.0, 0.5, 0.2);
		let foot_color = Color::rgb(0.8, 0.7, 0.4);
		let eye_color = Color::rgb(0.2, 0.2, 0.2);

		self.mesh_data.clear();

		let body_vertices = [
			self.player_hat_pos,
			body_transform * Vec3::new(-1.0, 0.0,-1.0),
			body_transform * Vec3::new(-1.0, 0.0, 1.0),
			body_transform * Vec3::new( 1.0, 0.0, 1.0),
			body_transform * Vec3::new( 1.0, 0.0,-1.0),
		];

		let mut mb = ColorMeshBuilder::new(&mut self.mesh_data);
		mb.set_color(body_color);
		mb.extend_3d_fan_closed(5, body_vertices);
		mb.extend_3d_fan(4, body_vertices[1..].iter().rev().cloned());

		for &foot_pos in player.feet_positions.iter() {
			// TODO(pat.m): take orientation from scene intersection
			let foot_plane = Mat3::from_columns([
				Vec3::from_x(1.0),
				Vec3::from_z(-1.0),
				foot_pos + Vec3::from_y(0.1),
			]);

			let mut pmb = mb.on_plane_ref(foot_plane);
			pmb.set_color(foot_color);
			pmb.build(geom::Polygon::from_matrix(9, Mat2x3::uniform_scale(foot_size)));
		}

		let front_face_center = (body_vertices[1] + body_vertices[4]) / 2.0;
		let front_face_up = self.player_hat_pos - front_face_center;
		let front_face_right = body_vertices[1] - body_vertices[4];
		let front_face_away = front_face_right.cross(front_face_up).normalize();

		let front_face_plane = Mat3::from_columns([
			front_face_right.normalize(),
			front_face_up.normalize(),
			front_face_center + front_face_up / 2.0 + front_face_away * 0.1,
		]);

		self.blink_timer += 1.0 / 60.0;
		self.blink_timer %= 3.0;

		let eye_scale = if self.blink_timer < 0.1 {
			Vec2::new(0.5, 0.1)
		} else {
			Vec2::new(0.4, 0.4)
		};

		let mut pmb = mb.on_plane_ref(front_face_plane);
		pmb.set_color(eye_color);
		pmb.build(geom::Polygon::from_pos_scale(9, Vec2::from_x(-0.4), eye_scale));
		pmb.build(geom::Polygon::from_pos_scale(9, Vec2::from_x(0.4), eye_scale));

		self.mesh.upload(&self.mesh_data);

		// TODO(pat.m): move to model and controller
		let position_diff = player.body_position - self.player_hat_pos + Vec3::from_y(2.0);
		self.player_vel *= 0.92;
		self.player_vel += position_diff * 0.05;
		self.player_hat_pos += self.player_vel;
	}


	#[instrument(skip_all)]
	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("player");

		ctx.gfx.bind_shader(self.shader);
		self.mesh.draw(&mut ctx.gfx, gfx::DrawMode::Triangles);
	}
}