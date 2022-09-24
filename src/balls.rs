use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};
use crate::shaders;

use toybox::input::raw::{Scancode, MouseButton};

toybox::declare_input_context! {
	struct Actions "Balls" {
		state forward { "Forward" [Scancode::W] }
		state back { "Back" [Scancode::S] }
		state left { "Left" [Scancode::A] }
		state right { "Right" [Scancode::D] }
		state sprint { "Sprint" [Scancode::LShift] }
		state crouch { "Crouch" [Scancode::LCtrl] }
		trigger spawn_ball { "Spawn Ball" [MouseButton::Left] }
		mouse mouse { "Mouse" [1.0] }

		trigger toggle_debug { "Debug" [Scancode::Grave] }
	}
}

toybox::declare_input_context! {
	struct DebugMouseActions "DebugMouse" {
		pointer mouse { "Mouse" }
	}
}


const ZONE_SIZE: f32 = 3.0;


pub async fn play() -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = crate::platformer3d::controller::GlobalController::new(&mut engine, resource_scope_token.id())?;

	let actions = Actions::new_active(&mut engine);
	let debug_mouse_actions = DebugMouseActions::new(&mut engine);

	let mut resource_ctx = engine.gfx.resource_context(&resource_scope_token);
	let mut mesh = gfx::Mesh::new(&mut resource_ctx);
	let mut mesh_data: gfx::MeshData<gfx::ColorVertex> = gfx::MeshData::new();

	let shader = resource_ctx.new_simple_shader(shaders::COLOR_3D_VERT, shaders::FLAT_COLOR_FRAG)?;
	let mut uniform_buffer = resource_ctx.new_buffer(gfx::BufferUsage::Stream);

	let mut camera = Camera {
		pos: Vec2::new(0.0, 5.0),
		elevation: 1.0,
		yaw: 0.0,
		pitch: (1.0/5.0f32).atan(),
	};

	let mut move_speed = 0.0;
	let mut time = 0.0f32;



	struct Ball {
		pos: Vec3,
		vel: Vec3,
		radius: f32,
		bounce_elapsed: f32,
	}

	let mut rng = thread_rng();
	let mut balls = Vec::new();

	for _ in 0..250 {
		balls.push(Ball {
			pos: (random::<Vec3>() * 2.0 - 1.0) * ZONE_SIZE + Vec3::from_y(2.0),
			vel: (random::<Vec3>() * 2.0 - 1.0) * 5.0,
			radius: rng.gen_range::<f32, _>(0.2..0.7).powf(1.5),
			bounce_elapsed: 10.0,
		})
	}


	let mixer_id = engine.audio.update_graph_immediate(|graph| {
		let node = audio::nodes::MixerNode::new(0.3);
		let node_id = graph.add_node(node, graph.output_node());
		graph.pin_node_to_scope(node_id, &resource_scope_token);
		node_id
	});


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
		}


		// Camera control
		{
			let frame_state = engine.input.frame_state();

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
				let world_pos = camera.pos.to_x0z() + Vec3::from_y(camera.elevation);
				let camera_orientation = camera_orientation
					* Quat::from_pitch(camera.pitch);

				let spawn_pos = world_pos + camera_orientation.forward() * 0.6;
				let vel = camera_orientation.forward() * rng.gen_range(0.4..3.0);

				balls.push(Ball {
					pos: spawn_pos,
					vel,
					radius: rng.gen_range::<f32, _>(0.1..0.8).powf(1.5),
					bounce_elapsed: 10.0,
				})
			}
		}


		// Update balls
		for ball in balls.iter_mut() {
			// gravity
			ball.vel.y -= 2.0/60.0;

			ball.pos += ball.vel * 1.0/60.0;

			ball.bounce_elapsed += 1.0/60.0;

			let planes = [
				Plane::new(Vec3::from_y(1.0), 0.0),
				Plane::new(Vec3::from_y(-1.0), -6.0),

				Plane::new(Vec3::from_x(1.0), -ZONE_SIZE),
				Plane::new(Vec3::from_x(-1.0), -ZONE_SIZE),
				Plane::new(Vec3::from_z(1.0), -ZONE_SIZE),
				Plane::new(Vec3::from_z(-1.0), -ZONE_SIZE),
			];

			let player_diff = ball.pos.to_xz() - camera.pos;
			let player_radius = 0.5;
			let player_dist = player_diff.length();
			let surface_dist = player_dist - (ball.radius + player_radius);

			if surface_dist < 0.0 && ball.pos.y < 2.0 {
				let offset = (player_diff / player_dist).to_x0z() * -surface_dist;
				ball.pos += offset;

				let impact = offset * 3.0;

				if ball.vel.dot(offset) < 0.0 {
					ball.vel = (-ball.vel.to_xz()).to_x0z() + impact;
				} else {
					ball.vel += impact;
				}

				ball.vel.y += -8.0*surface_dist;
				ball.bounce_elapsed = 0.0;
			}

			// Wall bounces
			for plane in planes {
				let distance = plane.distance_to(ball.pos);
				let surface_dist = distance - ball.radius;

				if surface_dist < 0.0 {
					let absorbtion_factor = 0.9;
					let impact_speed = plane.normal.dot(ball.vel);
					ball.pos += plane.normal * -surface_dist;
					ball.vel -= plane.normal * impact_speed * 2.0 * absorbtion_factor;


					if impact_speed.abs() > 0.05 && ball.bounce_elapsed > 0.02 {
						let radius = ball.radius;

						engine.audio.queue_update(move |graph| {
							use audio::*;

							let impact_gain = impact_speed.abs() * radius;
							let dist_falloff = 1.0 / player_dist.powi(2);
							let gain = (impact_gain.powi(2)*dist_falloff).min(1.0);

							let freq = 100.0 / radius.powf(1.0);

							let node = audio::node_builder::OscillatorGenerator::new(freq)
								.envelope(0.01, 0.3)
								.gain(gain)
								.build();

							if gain > 0.0001 {
								graph.add_node(node, mixer_id);
							}
						});

						ball.bounce_elapsed = 0.0;
					}
				}
			}
		}


		use gfx::geom::*;

		mesh_data.clear();

		let mut mb = gfx::ColorMeshBuilder::new(&mut mesh_data);

		let ground_plane = Mat3::from_columns([
			Vec3::from_x(ZONE_SIZE * 2.0),
			Vec3::from_z(ZONE_SIZE * 2.0),
			Vec3::zero(),
		]);

		mb.set_color(Color::hsv(200.0, 0.35, 0.6));
		mb.on_plane_ref(ground_plane).build(Quad::unit());

		let quat = Quat::from_yaw(camera.yaw)
			* Quat::from_pitch(camera.pitch);

		// A Mat3x2 would be handy here.
		let camera_plane = Mat3::from_columns([
			quat.up(),
			quat.right(),
			Vec3::zero(),
		]);

		for ball in balls.iter() {
			let bounce_factor = (1.0 - ball.bounce_elapsed / 1.0).max(0.0);
			let scale = 2.0 * ball.radius * (1.0 - bounce_factor.powi(10)*0.05);
			let color = Color::hsv(30.0 - bounce_factor*20.0, 0.7 + bounce_factor*0.1, 0.6 - bounce_factor*0.1);

			draw_billboard(&mut mb, camera_plane, Polygon::from_matrix(13, Mat2x3::uniform_scale(scale)), ball.pos, color);
		}


		let ground_plane = Mat3::from_columns([
			Vec3::from_x(1.0),
			Vec3::from_z(1.0),
			Vec3::from_y(0.01),
		]);

		mb.set_color(Color::hsv(210.0, 0.38, 0.5));
		let mut ground_mb = mb.on_plane_ref(ground_plane);

		for ball in balls.iter() {
			let txform = Mat2x3::scale_translate(Vec2::splat(2.0 * ball.radius / (ball.pos.y - ball.radius + 1.0).max(0.0)), ball.pos.to_xz());
			ground_mb.build(Polygon::from_matrix(13, txform))
		}

		mesh.upload(&mesh_data);


		let uniforms = build_uniforms(&camera, engine.gfx.aspect());
		uniform_buffer.upload_single(&uniforms);

		let mut gfx = engine.gfx.draw_context();
		gfx.set_backface_culling(false);
		gfx.set_clear_color(Color::hsv(190.0, 0.3, 0.8));
		gfx.clear(gfx::ClearMode::ALL);

		gfx.bind_uniform_buffer(0, uniform_buffer);
		gfx.bind_shader(shader);

		mesh.draw(&mut gfx, gfx::DrawMode::Triangles);

		engine = next_frame(engine).await;
	}

	Ok(())
}


fn draw_billboard<MB>(mb: &mut MB, mut plane: Mat3, geom: impl gfx::traits::BuildableGeometry2D, pos: Vec3, color: Color)
	where MB: PolyBuilder3D + ColoredPolyBuilder
{
	let Vec3{x, y, z} = pos;
	plane.rows[0].z = x;
	plane.rows[1].z = y;
	plane.rows[2].z = z;

	mb.set_color(color);
	mb.on_plane_ref(plane).build(geom);
}


struct Camera {
	pos: Vec2,
	elevation: f32,

	yaw: f32,
	pitch: f32,
}




#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
	projection_view: Mat4,
	projection_view_inverse: Mat4,
	ui_projection_view: Mat4,
	// NOTE: align to Vec4s
}


fn build_uniforms(camera: &Camera, aspect: f32) -> Uniforms {
	let eye_pos = camera.pos.to_x0z() + Vec3::from_y(camera.elevation);

	let projection_view = {
		Mat4::perspective(PI/3.0, aspect, 0.1, 1000.0)
			* Mat4::rotate_x(-camera.pitch)
			* Mat4::rotate_y(-camera.yaw)
			* Mat4::translate(-eye_pos)
	};

	Uniforms {
		projection_view,
		projection_view_inverse: projection_view.inverse(),

		ui_projection_view: Mat4::identity()
	}
}

