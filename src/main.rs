use toybox::prelude::*;
use std::error::Error;

mod views;
mod model;
mod controller;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	engine.gl_ctx.add_shader_import("global", include_str!("shaders/global.common.glsl"));

	let uniform_buffer = engine.gl_ctx.new_buffer();
	engine.gl_ctx.bind_uniform_buffer(0, uniform_buffer);


	let cube_view = views::CubeView::new(&engine.gl_ctx)?;
	let mut perf_view = views::PerfView::new(&engine.gl_ctx)?;
	let mut player_view = views::PlayerView::new(&engine.gl_ctx)?;

	let mut player = model::Player::new();
	let mut camera = model::Camera::new();

	let mut global_controller = controller::GlobalController::new(&mut engine)?;
	let mut player_controller = controller::PlayerController::new(&mut engine.input);


	for input_context in engine.input.contexts() {
		dbg!(input_context);
	}

	'main: loop {
		engine.process_events();

		if engine.should_quit() {
			break 'main
		}

		global_controller.update(&mut engine);

		if global_controller.should_quit() {
			break 'main
		}

		{
			let Vec2{x, y} = engine.gl_ctx.canvas_size().to_vec2();
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

		uniform_buffer.upload(&[uniforms], gl::BufferUsage::Stream);


		perf_view.update(&engine.instrumenter, camera.aspect);
		player_view.update(&player);

		unsafe {
			gl::raw::ClearColor(0.1, 0.1, 0.1, 1.0);
			gl::raw::Clear(gl::raw::COLOR_BUFFER_BIT | gl::raw::DEPTH_BUFFER_BIT);
		}

		let mut view_ctx = views::ViewContext::new(&engine.gl_ctx, &mut engine.instrumenter);

		cube_view.draw(&mut view_ctx);
		player_view.draw(&mut view_ctx);
		perf_view.draw(&mut view_ctx);

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

