use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};
use crate::shaders;

use toybox::input::raw::{Scancode, MouseButton};

use std::num::Wrapping;

toybox::declare_input_context! {
	struct Actions "Balls" {
		state forward { "Forward" [Scancode::W] }
		state back { "Back" [Scancode::S] }
		state left { "Left" [Scancode::A] }
		state right { "Right" [Scancode::D] }
		state sprint { "Sprint" [Scancode::LShift] }
		state crouch { "Crouch" [Scancode::LCtrl] }
		state spawn_ball { "Spawn Ball" [MouseButton::Left] }
		trigger remove_balls { "Remove Balls" [Scancode::Space] }
		trigger throw_grenade { "Throw Grenade" [MouseButton::Right] }
		mouse mouse { "Mouse" [1.0] }

		trigger toggle_debug { "Debug" [Scancode::Grave] }
	}
}

toybox::declare_input_context! {
	struct DebugMouseActions "DebugMouse" {
		pointer mouse { "Mouse" }
	}
}


const ZONE_SIZE: f32 = 6.0;


pub async fn play() -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = crate::global_controller::GlobalController::new(&mut engine, resource_scope_token.id())?;

	let actions = Actions::new_active(&mut engine);
	let debug_mouse_actions = DebugMouseActions::new(&mut engine);

	let mut resource_ctx = engine.gfx.resource_context(&resource_scope_token);
	let mut mesh = gfx::Mesh::new(&mut resource_ctx);
	let mut mesh_data: gfx::MeshData<PatternVertex> = gfx::MeshData::new();

	let shader = resource_ctx.new_simple_shader(include_str!("../shaders/balls_pattern.vert.glsl"), include_str!("../shaders/balls_pattern.frag.glsl"))?;
	let mut std_uniform_buffer = resource_ctx.new_buffer(gfx::BufferUsage::Stream);
	let mut pattern_uniform_buffer = resource_ctx.new_buffer(gfx::BufferUsage::Stream);

	let mut camera = Camera {
		pos: Vec2::new(0.0, 5.0),
		elevation: 1.0,
		yaw: 0.0,
		pitch: (1.0/5.0f32).atan(),
	};

	let mut move_speed = 0.0;
	let mut time = 0.0f32;



	let mut rng = thread_rng();
	let mut balls = Vec::new();

	for _ in 0..1 {
		balls.push(Ball {
			pos: (random::<Vec3>() * 2.0 - 1.0) * ZONE_SIZE + Vec3::from_y(2.0),
			radius: rng.gen_range::<f32, _>(0.2..0.7).powf(1.5),

			ty: BallType::Bouncy {
				vel: (random::<Vec3>() * 2.0 - 1.0) * 5.0,
				hue: rng.gen_range(0.0..360.0),
				bounce_elapsed: 10.0,
			}
		})
	}


	let mixer_id = engine.audio.update_graph_immediate(|graph| {
		let node = audio::nodes::MixerNode::new(0.5);
		let node_id = graph.add_node(node, graph.output_node());
		graph.pin_node_to_scope(node_id, &resource_scope_token);
		node_id
	});


	let mut background_color = Color::hsv(67.68, 0.216, 1.0).to_linear();
	let mut ground_color_0 = Color::hsv(4.3, 0.259, 1.0).to_linear();
	let mut ground_color_1 = Color::hsv(275.4, 0.188, 1.0).to_linear();
	let mut shadow_color = Color::hsv(345.96, 0.29, 0.855).to_linear();

	let mut grenade_color = Color::hsv(0.0, 0.21, 0.36).to_linear();

	loop {
		time += 1.0 / 60.0;

		global_controller.update(&mut engine);
		if global_controller.should_quit() {
			break
		}


		// Toggle free mouse
		if engine.input.frame_state().active(actions.toggle_debug) {
			let currently_active = engine.input.is_context_active(debug_mouse_actions.context_id());
			engine.input.set_context_active(debug_mouse_actions.context_id(), !currently_active);
			engine.imgui.set_visible(!currently_active);
			engine.imgui.set_input_enabled(!currently_active);
		}

		{
			let colors = [
				("background", &mut background_color),
				("ground 0", &mut ground_color_0),
				("ground 1", &mut ground_color_1),
				("drop shadow", &mut shadow_color),
				("grenade", &mut grenade_color),
			];

			for (name, color) in colors {
				let Color { r, g, b, .. } = color.to_srgb();
				let mut color_raw = [r, g, b];
				if imgui::ColorEdit::new(name, &mut color_raw)
					.build(engine.imgui.frame())
				{
					*color = Color::from(color_raw).to_linear();
				}
			}

		}


		// Camera control
		{
			let Engine { input, audio, .. } = &mut *engine;
			let frame_state = input.frame_state();

			if let Some(mouse) = frame_state.mouse(actions.mouse) {
				let (pitch_min, pitch_max) = (-PI/2.0, PI/2.0);

				camera.yaw -= mouse.x * 0.5;
				camera.pitch = (camera.pitch + mouse.y as f32 * 0.5).clamp(pitch_min, pitch_max);
			}

			let camera_orientation = Quat::from_yaw(camera.yaw);
			let mut move_direction = Vec3::zero();

			if frame_state.active(actions.forward) { move_direction += camera_orientation.forward() }
			if frame_state.active(actions.back) { move_direction -= camera_orientation.forward() }
			if frame_state.active(actions.left) { move_direction -= camera_orientation.right() }
			if frame_state.active(actions.right) { move_direction += camera_orientation.right() }

			if move_direction.length() > 0.1 {
				let base_move_speed = match frame_state.active(actions.sprint) {
					true => 12.0,
					false => 5.0,
				};

				move_speed += (base_move_speed - move_speed) * 4.0 / 60.0;
				move_direction = move_direction.normalize();
			} else {
				move_speed *= 0.8;
			}

			camera.pos += move_direction.to_xz() * (move_speed / 60.0);

			let target_elevation = match frame_state.active(actions.crouch) {
				true => 0.4,
				false => 1.0,
			};

			camera.elevation = (0.4).lerp(camera.elevation, target_elevation);

			if frame_state.active(actions.spawn_ball) {
				let world_pos = camera.pos.to_x0y() + Vec3::from_y(camera.elevation);
				let camera_orientation = camera_orientation
					* Quat::from_pitch(camera.pitch);

				let spawn_pos = world_pos + camera_orientation.forward() * 0.6;
				let vel = camera_orientation.forward() * rng.gen_range(0.4..3.0);

				balls.push(Ball {
					pos: spawn_pos,
					radius: rng.gen_range::<f32, _>(0.05..1.1),

					ty: BallType::Bouncy {
						vel,
						hue: rng.gen_range(0.0..360.0),
						bounce_elapsed: 10.0,
					}
				})
			}

			if frame_state.active(actions.remove_balls) {
				let eye_pos = camera.pos.to_x0y() + Vec3::from_y(camera.elevation);

				for ball in balls.iter_mut()
					.filter(|ball| match ball.ty {
						BallType::Bouncy{..} => (ball.pos - eye_pos).length() < 5.0,
						_ => false
					})
				{
					ball.ty = BallType::Popping {
						elapsed: -((ball.pos - eye_pos).length() / 5.0 * 0.8)
					};
				}

				audio.queue_update(move |graph| {
					use audio::*;
					use audio::generator as gen;
					use audio::envelope as env;

					let fizz_cutoff = 600.0;

					let fizz = gen::Noise::new()
						.low_pass(fizz_cutoff)
						.low_pass(fizz_cutoff)
						.envelope(env::AR::new(0.01, 1.8).exp(4.0));

					let bong_1 = gen::GeneratorNode::new_sine(80.0);
					let bong_2 = gen::GeneratorNode::new_sine(150.0).gain(0.5);
					let bong_3 = gen::GeneratorNode::new_sine(200.0).gain(0.35);

					let bong = (bong_1, bong_2, bong_3).add()
						.envelope(env::AR::new(0.3, 0.9).exp(4.0));

					let node = (fizz, bong).add()
						.gain(0.9)
						.build();

					graph.add_node(node, mixer_id);
				});
			}

			if frame_state.active(actions.throw_grenade) {
				let eye_pos = camera.pos.to_x0y() + Vec3::from_y(camera.elevation);
				let camera_orientation = camera_orientation
					* Quat::from_pitch(camera.pitch);

				let spawn_pos = eye_pos + camera_orientation.forward() * 1.0;
				let vel = camera_orientation.forward() * 8.0;

				balls.push(Ball {
					pos: spawn_pos,
					radius: 0.2,

					ty: BallType::Grenade {
						vel,
						countdown: 3.0,
						bounce_elapsed: 10.0,
					}
				})
			}
		}


		update_balls(&mut engine, &mut balls, &camera, mixer_id);


		use gfx::geom::*;

		mesh_data.clear();

		let mut mb = PatternMeshBuilder::new(&mut mesh_data);


		// Ground plane
		let mut gmb = mb.on_plane_ref(gfx::OrthogonalOrientation::PositiveY);
		gmb.set_colors(ground_color_0, ground_color_1);
		gmb.pattern = 3;
		gmb.build(Quad::unit().uniform_scale(2.0 * ZONE_SIZE));


		// Billboards
		let quat = Quat::from_yaw(camera.yaw)
			* Quat::from_pitch(camera.pitch);

		let mut bmb = mb.on_plane_ref(quat);

		for ball in balls.iter() {
			bmb.set_origin(ball.pos);

			match ball.ty {
				BallType::Bouncy { bounce_elapsed, hue, .. } => {
					let bounce_factor = (1.0 - bounce_elapsed / 0.2).max(0.0);
					let scale = 2.0 * ball.radius;

					let color = Color::hsv(hue, 0.7, 0.8 + bounce_factor*0.2);
					let color2 = Color::hsv(hue + 50.0, 0.6, 0.85);

					bmb.set_colors(color, color2);
					bmb.pattern = 4;
					bmb.build(Polygon::unit(18).uniform_scale(scale));
				}

				BallType::Popping { elapsed } => {
					let pop_amt = (elapsed / BALL_POP_TIME).clamp(0.0, 1.0);
					let whiteness = (pop_amt + 0.4).clamp(0.2, 1.0).powi(2);

					let scale = (2.0 + pop_amt.powi(10)) * ball.radius;
					let color = Color::rgb(1.0, whiteness, whiteness);
					let color2 = Color::rgb(1.0, 0.3, 0.3);

					bmb.set_colors(color, color2);
					bmb.pattern = 2;
					bmb.build(Polygon::unit(18).uniform_scale(scale));
				}

				BallType::Grenade { countdown, .. } => {
					let scale;
					bmb.pattern = 2;

					if countdown > GRENADE_TEASE_TIME {
						scale = 2.0 * ball.radius;
						bmb.set_color(grenade_color);
					} else {
						let explode_amt = 1.0 - (countdown / GRENADE_TEASE_TIME).clamp(0.0, 1.0);
						let whiteness = (explode_amt + 0.4).clamp(0.2, 1.0).powi(2);
						scale = (2.0 + 4.0 * explode_amt.powi(6)) * ball.radius;
						let color = Color::rgb(1.0, whiteness, whiteness);
						let color2 = Color::rgb(1.0, 0.3, 0.3);

						bmb.set_colors(color, color2);
					}

					bmb.build(Polygon::unit(18).uniform_scale(scale));
				}
			}
		}


		// Drop shadows
		let mut gmb = mb.on_plane_ref(gfx::OrthogonalOrientation::PositiveY.to_surface().with_origin(Vec3::from_y(0.001)));

		gmb.set_color(shadow_color);
		gmb.pattern = 0;

		for ball in balls.iter() {
			let scale = 2.0 * ball.radius / (ball.pos.y - ball.radius + 1.0).max(0.0);
			let pos = ball.pos.to_xz();
			gmb.build(Polygon::unit(13).uniform_scale(scale).translate(pos));
		}

		mesh.upload(&mesh_data);


		#[repr(C)]
		#[derive(Copy, Clone, Debug)]
		struct PatternUniforms {
			screen_dimensions: Vec2, //_pad2: [f32; 2],
			time: f32, _pad: [f32; 3],
		}

		let uniforms = build_uniforms(&camera, engine.gfx.aspect());
		std_uniform_buffer.upload_single(&uniforms);
		pattern_uniform_buffer.upload_single(&PatternUniforms {
			screen_dimensions: engine.gfx.backbuffer_size().to_vec2(),
			time,

			_pad: <_>::default(),
			// _pad2: <_>::default(),
		});

		let mut gfx = engine.gfx.draw_context();
		gfx.set_backface_culling(false);
		gfx.set_clear_color(background_color);
		gfx.clear(gfx::ClearMode::ALL);

		gfx.bind_uniform_buffer(0, std_uniform_buffer);
		gfx.bind_uniform_buffer(1, pattern_uniform_buffer);
		gfx.bind_shader(shader);

		mesh.draw(&mut gfx, gfx::DrawMode::Triangles);

		engine = next_frame(engine).await;
	}

	Ok(())
}


struct Camera {
	pos: Vec2,
	elevation: f32,

	yaw: f32,
	pitch: f32,
}




fn build_uniforms(camera: &Camera, aspect: f32) -> shaders::StdUniforms {
	let eye_pos = camera.pos.to_x0y() + Vec3::from_y(camera.elevation);

	let projection_view = {
		Mat4::perspective(PI/3.0, aspect, 0.1, 1000.0)
			* Mat4::rotate_x(-camera.pitch)
			* Mat4::rotate_y(-camera.yaw)
			* Mat4::translate(-eye_pos)
	};

	shaders::StdUniforms {
		projection_view,
		projection_view_inverse: projection_view.inverse(),

		ui_projection_view: Mat4::identity()
	}
}






const BALL_POP_TIME: f32 = 0.4;
const GRENADE_TEASE_TIME: f32 = 0.5;

#[derive(Copy, Clone)]
enum BallType {
	Bouncy {
		vel: Vec3,
		hue: f32,
		bounce_elapsed: f32,
	},

	Popping {
		elapsed: f32,
	},

	Grenade {
		vel: Vec3,
		countdown: f32,
		bounce_elapsed: f32,
	},
}

struct Ball {
	pos: Vec3,
	radius: f32,
	ty: BallType,
}



struct Bounce {
	impact_speed: f32,
}

fn bounce(ball_pos: &mut Vec3, ball_vel: &mut Vec3, ball_radius: f32, eye_pos: Vec3) -> Option<Bounce> {
	let planes = [
		Plane::new(Vec3::from_y(1.0), 0.0),
		Plane::new(Vec3::from_y(-1.0), -6.0),

		Plane::new(Vec3::from_x(1.0), -ZONE_SIZE),
		Plane::new(Vec3::from_x(-1.0), -ZONE_SIZE),
		Plane::new(Vec3::from_z(1.0), -ZONE_SIZE),
		Plane::new(Vec3::from_z(-1.0), -ZONE_SIZE),
	];

	let player_diff = *ball_pos - eye_pos;
	let player_diff = Vec3 { y: player_diff.y.max(0.0), .. player_diff};

	let player_radius = 0.5;
	let player_dist = player_diff.length();
	let surface_dist = player_dist - (ball_radius + player_radius);

	let mut total_impact_speed = 0.0;

	if surface_dist < 0.0 && ball_pos.y < 2.0 {
		let impact_normal = player_diff / player_dist;
		let offset = impact_normal * -surface_dist;
		*ball_pos += offset;

		let impact = offset * 3.0;
		let impact_speed = -impact_normal.dot(*ball_vel);

		if ball_vel.dot(offset) < 0.0 {
			*ball_vel = (-ball_vel.to_xz()).to_x0y() + impact;
		} else {
			*ball_vel += impact;
		}

		ball_vel.y += -8.0*surface_dist;

		total_impact_speed += impact_speed.max(0.0);
	}

	// Wall bounces
	for plane in planes {
		let distance = plane.distance_to(*ball_pos);
		let penetration_dist = ball_radius - distance;

		if penetration_dist > 0.0 {
			let absorbtion_factor = 0.7;
			*ball_pos += plane.normal * penetration_dist;

			let impact_speed = -plane.normal.dot(*ball_vel);

			// Zero velocity in this direction
			let impulse_vector = plane.normal * impact_speed;
			*ball_vel += impulse_vector;

			if impact_speed > 0.1 && penetration_dist > 0.001 {
				// Reflect and record impact
				*ball_vel += impulse_vector * absorbtion_factor;
				total_impact_speed += impact_speed.max(0.0);
			}
		}
	}

	if total_impact_speed > 0.0 {
		Some(Bounce {
			impact_speed: total_impact_speed,
		})
	} else {
		None
	}
}


fn update_balls(engine: &mut toybox::Engine, balls: &mut Vec<Ball>, camera: &Camera, mixer_id: audio::NodeId) {
	use audio::generator as gen;
	use audio::envelope as env;

	let mut rng = thread_rng();

	let eye_pos = camera.pos.to_x0y() + Vec3::from_y(camera.elevation);
	let mut explosion_point = None;

	for ball in balls.iter_mut() {
		match &mut ball.ty {
			BallType::Bouncy { vel, bounce_elapsed, .. } => {
				// gravity
				if ball.pos.y > ball.radius {
					vel.y -= 6.0/60.0;
				}

				ball.pos += *vel * 1.0/60.0;

				// *vel *= (1.0 - 0.5/60.0);

				*bounce_elapsed += 1.0/60.0;

				if let Some(Bounce {impact_speed}) = bounce(&mut ball.pos, vel, ball.radius, eye_pos) {
					if impact_speed > 0.05 && *bounce_elapsed > 0.1  {
						let impact_gain = impact_speed * ball.radius.sqrt() * 0.3;
						let eye_dist = (ball.pos - eye_pos).length();
						let dist_falloff = 1.0 / eye_dist.powi(2);
						let gain = (impact_gain.powi(1)*dist_falloff).min(1.0);

						let freq = 80.0 * 2.0f32.powf((1.0 + 3.0 / ball.radius.sqrt()).floor() / 9.0);

						let release = 0.1 + ball.radius / 3.0;

						if gain > 0.0001 {
							engine.audio.queue_update(move |graph| {
								use audio::*;

								let low_osc = gen::GeneratorNode::new_triangle(freq)
									.envelope(env::AR::new(0.01, release).exp(4.0))
									.gain(4.0);

								let high_osc = (gen::GeneratorNode::new_sine(freq * 2.0), gen::Noise::new().low_pass(100.0))
									.add()
									.envelope(env::AR::new(0.01, 0.08).exp(4.0))
									.gain(0.3);

								let node = (low_osc, high_osc)
									.add()
									.gain(gain)
									.build();

								graph.add_node(node, mixer_id);
							});
						}

						*bounce_elapsed = 0.0;
					}
				}
			}

			BallType::Popping { elapsed } => {
				*elapsed += 1.0/60.0;

				if *elapsed >= BALL_POP_TIME {
					let eye_dist = (ball.pos - camera.pos.to_x0y() - Vec3::from_y(camera.elevation)).length();
					let gain = 2.0 * ball.radius / eye_dist.powi(1);

					let freq = rng.gen_range(600.0 .. 800.0);

					engine.audio.queue_update(move |graph| {
						use audio::*;

						let noise = gen::Noise::new()
							.low_pass(4000.0);

						let osc = gen::GeneratorNode::new_triangle(freq);

						let node = (noise, osc).add()
							.envelope(env::AR::new(0.01, 0.4).exp(4.0))
							.gain(gain)
							.build();

						graph.add_node(node, mixer_id);
					});
				}
			}

			BallType::Grenade { countdown, bounce_elapsed, vel } => {
				*countdown -= 1.0/60.0;
				*bounce_elapsed += 1.0/60.0;

				if *countdown > GRENADE_TEASE_TIME {
					// gravity
					if ball.pos.y > ball.radius + 0.01 {
						vel.y -= 9.0/60.0;
					}

					ball.pos += *vel / 60.0;

					if let Some(Bounce {impact_speed}) = bounce(&mut ball.pos, vel, ball.radius, eye_pos) {
						if impact_speed > 0.05 && *bounce_elapsed > 0.1 {
							let impact_gain = impact_speed * ball.radius.sqrt() * 0.3;
							let eye_dist = (ball.pos - eye_pos).length();
							let dist_falloff = 1.0 / eye_dist.powi(2);
							let gain = (impact_gain.powi(2)*dist_falloff).min(1.0);

							let freq = 2000.0;

							let release = 0.1;

							if gain > 0.0001 {
								engine.audio.queue_update(move |graph| {
									use audio::*;
									let node = gen::GeneratorNode::new_sine(freq)
										.envelope(env::AR::new(0.01, release).exp(4.0))
										.gain(gain)
										.build();

									graph.add_node(node, mixer_id);
								});
							}
						}

						*bounce_elapsed = 0.0;
					}
				} else if *countdown < 0.0 {
					if explosion_point.is_some() {
						*countdown = 0.0;
					} else {
						explosion_point = Some(ball.pos);
					}
				}
			}
		}
	}

	if let Some(explosion_point) = explosion_point {
		let dist_falloff = 1.0 / (explosion_point - eye_pos).length().powi(1);
		let gain = 4.0 * dist_falloff;

		engine.audio.queue_update(move |graph| {
			use audio::*;

			let hi_noise = gen::Noise::new()
				.high_pass(5000.0)
				.gain(2.0)
				.envelope(env::AR::new(0.01, 0.6).exp(4.0));

			let low_noise = gen::Noise::new()
				.low_pass(100.0)
				.gain(2.0)
				.envelope(env::AR::new(0.1, 0.8).exp(4.0));

			let osc = gen::GeneratorNode::new_triangle(50.0)
				.envelope(env::AR::new(0.1, 0.5).exp(4.0));

			let node = (hi_noise, low_noise, osc).add()
				.gain(gain)
				.build();

			graph.add_node(node, mixer_id);
		});

		for ball in balls.iter_mut() {
			let explosion_diff = ball.pos - explosion_point;
			let explosion_distance = explosion_diff.length() - ball.radius;
			let explosion_dir = explosion_diff.normalize() + Vec3::from_y(1.0);

			match &mut ball.ty {
				BallType::Bouncy{vel, ..} => if explosion_distance < 2.0 {
					// Pop
					ball.ty = BallType::Popping {
						elapsed: -((ball.pos - explosion_point).length() / 2.0 * 0.8)
					};
				} else if explosion_distance < 4.0 {
					// Push away
					let force = explosion_dir * (4.0 - explosion_distance) * 5.0;
					*vel += force;
				}

				BallType::Grenade {vel, ..} => if explosion_distance < 4.0 {
					// Push away
					let force = explosion_dir * (4.0 - explosion_distance) * 5.0;
					*vel += force;
				}

				_ => {}
			}
		}
	}

	balls.retain(|ball| {
		match ball.ty {
			BallType::Popping { elapsed } => elapsed < BALL_POP_TIME,
			BallType::Grenade { countdown, .. } => countdown >= 0.0,
			_ => true
		}
	});
}




use gfx::mesh::{MeshData, PolyBuilder2D};
use gfx::vertex::{*};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PatternVertex {
	pub pos: Vec3,
	pub index: u8,
	pub shape_id: u8,
	pub color_0: Color,
	pub color_1: Color,
}


impl Vertex for PatternVertex {
	fn descriptor() -> Descriptor {
		static COLOR_VERTEX_ATTRIBUTES: &'static [Attribute] = &[
			Attribute::new(0*4, AttributeType::Vec3),
			Attribute::new(3*4, AttributeType::Uint8(1)),
			Attribute::new(4*4, AttributeType::Vec4),
			Attribute::new(8*4, AttributeType::Vec4),
			Attribute::new(3*4 + 1, AttributeType::Uint8(1)),
		];

		Descriptor {
			attributes: COLOR_VERTEX_ATTRIBUTES,
			size_bytes: std::mem::size_of::<Self>() as u32,
		}
	}
}



pub struct PatternMeshBuilder<'md> {
	pub data: &'md mut MeshData<PatternVertex>,
	pub color_0: Color,
	pub color_1: Color,
	pub pattern: u8,
	pub shape_id: Wrapping<u8>,
}


impl<'md> PatternMeshBuilder<'md> {
	pub fn new(data: &'md mut MeshData<PatternVertex>) -> Self {
		PatternMeshBuilder {
			data,
			color_0: Color::white(),
			color_1: Color::black(),
			pattern: 0,
			shape_id: Wrapping(0),
		}
	}

	pub fn set_color(&mut self, color: Color) {
		self.color_0 = color;
		self.color_1 = color;
	}

	pub fn set_colors(&mut self, color_0: Color, color_1: Color) {
		self.color_0 = color_0;
		self.color_1 = color_1;
	}
}

impl<'mb> PolyBuilder3D for PatternMeshBuilder<'mb> {
	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>) {
		let shape_id = self.shape_id.0;
		self.shape_id += 1;

		let vertices = vs.into_iter()
			.map(|pos| PatternVertex {
				pos,
				index: self.pattern,
				shape_id,
				color_0: self.color_0,
				color_1: self.color_1,
			});

		self.data.extend(vertices, is);
	}
}