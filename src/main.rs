#![feature(array_chunks)]

use toybox::prelude::*;
use std::error::Error;

mod views;
mod model;
mod controller;
mod shaders;
mod mesh;

mod intersect;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	engine.gfx.add_shader_import("global", shaders::GLOBAL_COMMON);

	let mut uniform_buffer = engine.gfx.new_buffer();
	engine.gfx.bind_uniform_buffer(0, uniform_buffer);

	let mut player = model::Player::new();
	let mut camera = model::Camera::new();
	let mut debug_model = model::Debug::new();
	let mut scene = model::Scene::new()?;

	let mut blob_shadow_model = model::BlobShadowModel::new();

	let mut perf_view = views::PerfView::new(&engine.gfx)?;
	let mut player_view = views::PlayerView::new(&engine.gfx)?;
	let mut debug_view = views::DebugView::new(&engine.gfx)?;
	let mut scene_view = views::SceneView::new(&engine.gfx, &scene)?;
	let mut blob_shadow_view = views::BlobShadowView::new(&engine.gfx)?;

	let mut global_controller = controller::GlobalController::new(&mut engine)?;
	let mut player_controller = controller::PlayerController::new(&mut engine.input);
	let mut camera_controller = controller::CameraController::new(&mut engine.input);
	let mut gem_controller = controller::GemController::new(&mut engine.audio)?;
	let debug_controller = controller::DebugController::new(&mut engine.input);

	'main: loop {
		engine.process_events();

		if engine.should_quit() {
			break 'main
		}

		global_controller.update(&mut engine);

		if global_controller.should_quit() {
			break 'main
		}

		debug_controller.update(&mut engine.input, &mut debug_model, &mut scene);
		camera_controller.update(&mut engine.input, &mut camera, &player);
		player_controller.update(&mut engine.input, &mut player, &camera, &scene);
		gem_controller.update(&mut engine.audio, &player, &mut scene);

		blob_shadow_model.clear();

		perf_view.update(&engine.instrumenter, engine.gfx.aspect());
		debug_view.update(&debug_model);
		player_view.update(&player, &mut blob_shadow_model);
		scene_view.update(&scene, &mut blob_shadow_model);
		blob_shadow_view.update(&blob_shadow_model, &scene);

		engine.gfx.set_clear_color(Color::grey(0.1));
		engine.gfx.clear(gfx::ClearMode::ALL);

		let uniforms = build_uniforms(&camera, engine.gfx.aspect());
		uniform_buffer.upload(&[uniforms], gfx::BufferUsage::Stream);

		let mut view_ctx = views::ViewContext::new(&engine.gfx, &mut engine.instrumenter);

		scene_view.draw(&mut view_ctx);
		player_view.draw(&mut view_ctx);
		blob_shadow_view.draw(&mut view_ctx);

		if debug_model.active {
			perf_view.draw(&mut view_ctx);
			debug_view.draw(&mut view_ctx);
		}

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


fn build_uniforms(camera: &model::Camera, aspect: f32) -> Uniforms {
	Uniforms {
		projection_view: {
			let camera_orientation = Quat::from_pitch(-camera.pitch) * Quat::from_yaw(-camera.yaw);
			let camera_orientation = camera_orientation.to_mat4();

			Mat4::perspective(PI/3.0, aspect, 0.1, 1000.0)
				* camera_orientation
				* Mat4::translate(-camera.position)
		},

		ui_projection_view: {
			Mat4::scale(Vec3::new(1.0 / aspect, 1.0, 1.0))
		}
	}
}