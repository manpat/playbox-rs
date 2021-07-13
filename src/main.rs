use toybox::prelude::*;
use std::error::Error;

mod views;
mod model;
mod controller;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	dbg!(&engine.gfx.capabilities());

	engine.gfx.add_shader_import("global", include_str!("shaders/global.common.glsl"));

	let mut uniform_buffer = engine.gfx.new_buffer();
	engine.gfx.bind_uniform_buffer(0, uniform_buffer);

	let mut player = model::Player::new();
	let mut camera = model::Camera::new();
	let mut debug_model = model::Debug::new();
	let mut scene = model::Scene::new()?;

	let mut perf_view = views::PerfView::new(&engine.gfx)?;
	let mut player_view = views::PlayerView::new(&engine.gfx)?;
	let mut debug_view = views::DebugView::new(&engine.gfx)?;
	let mut scene_view = views::SceneView::new(&engine.gfx, &scene)?;

	let mut global_controller = controller::GlobalController::new(&mut engine)?;
	let mut player_controller = controller::PlayerController::new(&mut engine.input);
	let mut debug_controller = controller::DebugController::new(&mut engine.input);


	// for input_context in engine.input.contexts() {
	// 	dbg!(input_context);
	// }

	'main: loop {
		engine.process_events();

		if engine.should_quit() {
			break 'main
		}

		global_controller.update(&mut engine);

		if global_controller.should_quit() {
			break 'main
		}

		debug_controller.update(&mut engine.input, &mut debug_model);

		{
			let Vec2{x, y} = engine.gfx.canvas_size().to_vec2();
			camera.aspect = x / y;
		}

		player_controller.update(&mut engine.input, &mut player, &mut camera);

		let uniforms = Uniforms {
			projection_view: {
				let camera_orientation = Mat4::rotate_y(camera.yaw) * Mat4::rotate_x(camera.pitch);

				Mat4::perspective(PI/3.0, camera.aspect, 0.1, 1000.0)
					* Mat4::translate(Vec3::from_z(-camera.zoom))
					* camera_orientation.inverse()
					* Mat4::translate(-player.position)
			},

			ui_projection_view: {
				Mat4::scale(Vec3::new(1.0 / camera.aspect, 1.0, 1.0))
			}
		};

		uniform_buffer.upload(&[uniforms], gfx::BufferUsage::Stream);


		perf_view.update(&engine.instrumenter, camera.aspect);
		debug_view.update(&debug_model);
		player_view.update(&player);
		scene_view.update(&scene);

		unsafe {
			gfx::raw::ClearColor(0.1, 0.1, 0.1, 1.0);
			gfx::raw::Clear(gfx::raw::COLOR_BUFFER_BIT | gfx::raw::DEPTH_BUFFER_BIT);
		}

		let mut view_ctx = views::ViewContext::new(&engine.gfx, &mut engine.instrumenter);

		scene_view.draw(&mut view_ctx);
		player_view.draw(&mut view_ctx);
		perf_view.draw(&mut view_ctx);
		debug_view.draw(&mut view_ctx);

		engine.end_frame();
	}

	Ok(())
}




#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
	projection_view: Mat4,
	ui_projection_view: Mat4,
	// NOTE: align to Vec4s
}

