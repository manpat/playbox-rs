#![feature(array_chunks)]

use toybox::prelude::*;
use std::error::Error;

mod views;
mod model;
mod controller;
mod shaders;

mod intersect;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	engine.gfx.add_shader_import("global", shaders::GLOBAL_COMMON);

	let mut uniform_buffer = engine.gfx.new_buffer(gfx::BufferUsage::Stream);
	engine.gfx.render_state().bind_uniform_buffer(0, uniform_buffer);

	let mut player = model::Player::new();
	let mut camera = model::Camera::new();
	let mut debug_model = model::Debug::new();
	let mut scene = model::Scene::new()?;

	let mut blob_shadow_model = model::BlobShadowModel::new();

	let mut perf_view = views::PerfView::new(&mut engine.gfx)?;
	let mut player_view = views::PlayerView::new(&mut engine.gfx)?;
	let mut debug_view = views::DebugView::new(&mut engine.gfx)?;
	let mut scene_view = views::SceneView::new(&mut engine.gfx, &scene)?;
	let mut blob_shadow_view = views::BlobShadowView::new(&mut engine.gfx)?;
	let mut mesh_builder_test_view = views::MeshBuilderTestView::new(&mut engine.gfx)?;

	let mut global_controller = controller::GlobalController::new(&mut engine)?;
	let mut player_controller = controller::PlayerController::new(&mut engine);
	let mut camera_controller = controller::CameraController::new(&mut engine);
	let mut debug_camera_controller = controller::DebugCameraController::new(&mut engine);
	let mut gem_controller = controller::GemController::new(&mut engine)?;
	let debug_controller = controller::DebugController::new(&mut engine);

	let test_fbo = engine.gfx.new_framebuffer(
		gfx::FramebufferSettings::new(gfx::FramebufferSize::Backbuffer)
			.add_depth()
			.add_color(0, gfx::raw::R11F_G11F_B10F)
			.add_color(3, gfx::raw::RGBA8)
	);

	'main: loop {
		engine.process_events();

		if engine.should_quit() {
			break 'main
		}

		global_controller.update(&mut engine);

		if global_controller.should_quit() {
			break 'main
		}

		blob_shadow_model.clear();

		debug_controller.update(&mut engine, &mut debug_model, &mut scene, &mut camera);
		camera_controller.update(&mut engine, &mut camera, &player);
		debug_camera_controller.update(&mut engine, &mut camera);
		player_controller.update(&mut engine, &mut player, &mut blob_shadow_model, &camera, &scene);
		gem_controller.update(&mut engine, &mut scene, &player);

		perf_view.update(&engine.instrumenter, engine.gfx.aspect());
		debug_view.update(&debug_model);
		player_view.update(&player);
		scene_view.update(&scene, &mut blob_shadow_model);
		blob_shadow_view.update(&blob_shadow_model, &scene);
		mesh_builder_test_view.update();

		engine.gfx.render_state().set_clear_color(Color::grey(0.1));
		engine.gfx.render_state().clear(gfx::ClearMode::ALL);

		let uniforms = build_uniforms(&camera, engine.gfx.aspect());
		uniform_buffer.upload(&[uniforms]);

		let mut view_ctx = views::ViewContext::new(engine.gfx.render_state(), &mut engine.instrumenter);

		view_ctx.gfx.bind_framebuffer(&test_fbo);
		view_ctx.gfx.clear(gfx::ClearMode::ALL);

		scene_view.draw(&mut view_ctx);
		player_view.draw(&mut view_ctx);
		blob_shadow_view.draw(&mut view_ctx);
		view_ctx.gfx.bind_framebuffer(None);

		mesh_builder_test_view.draw(&mut view_ctx);


		if debug_model.active {
			perf_view.draw(&mut view_ctx);
			debug_view.draw(&mut view_ctx);
		}
			
		mesh_builder_test_view.draw_2d(&mut view_ctx);

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