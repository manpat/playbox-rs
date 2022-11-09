use toybox::prelude::*;

pub mod views;
pub mod model;
pub mod controller;

use crate::executor::{start_loop, next_frame};
use crate::shaders;



pub async fn load_and_play_scene(project_path: impl AsRef<std::path::Path>, scene_name: impl Into<String>) -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut player = model::Player::new();
	let mut camera = model::Camera::new();
	let mut debug_model = model::Debug::new();
	let mut scene = model::Scene::new(project_path, scene_name)?;

	let mut blob_shadow_model = model::BlobShadowModel::new();


	// TODO(pat.m): it would be cool to figure out how to tie a &mut Engine to a resource_scope_token
	let mut global_controller = crate::global_controller::GlobalController::new(&mut engine, resource_scope_token.id())?;
	let mut player_controller = controller::PlayerController::new(&mut engine, resource_scope_token.id());
	let mut camera_controller = controller::CameraController::new(&mut engine);
	let mut debug_camera_controller = controller::DebugCameraController::new(&mut engine);
	let mut gem_controller = controller::GemController::new(&mut engine, resource_scope_token.id())?;
	let mut audio_test_controller = controller::AudioTestController::new(&mut engine, &scene, resource_scope_token.id());
	let debug_controller = controller::DebugController::new(&mut engine);


	let mut view_resource_context = engine.gfx.resource_context(&resource_scope_token);
	let mut uniform_buffer = view_resource_context.new_buffer(gfx::BufferUsage::Stream);

	let mut player_view = views::PlayerView::new(&mut view_resource_context)?;
	let mut debug_view = views::DebugView::new(&mut view_resource_context, &scene)?;
	let mut scene_view = views::SceneView::new(&mut view_resource_context, &scene)?;
	let mut blob_shadow_view = views::BlobShadowView::new(&mut view_resource_context)?;
	let mut gbuffer_particles_view = views::GBufferParticlesView::new(&mut view_resource_context)?;

	let test_fbo = view_resource_context.new_framebuffer(
		gfx::FramebufferSettings::new(gfx::TextureSize::Backbuffer)
			.add_depth()
			.add_color(0, gfx::TextureFormat::R11G11B10F)
			.add_color(3, gfx::TextureFormat::color())
	);

	let test_fbo2 = view_resource_context.new_framebuffer(
		gfx::FramebufferSettings::new(gfx::TextureSize::BackbufferDivisor(3))
			.add_depth()
			.add_color(0, gfx::TextureFormat::color())
	);

	let post_effect_compute_shader = view_resource_context.new_compute_shader(shaders::TEST_POST_EFFECT_COMPUTE)?;

	let composite_shader = view_resource_context.new_simple_shader(shaders::FULLSCREEN_QUAD_VERT,
		include_str!("shaders/final_composite.frag.glsl"))?;


	'main: loop {
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
		audio_test_controller.update(&mut engine, &camera);

		debug_view.update(&engine, &debug_model);
		player_view.update(&player);
		scene_view.update(&scene, &mut blob_shadow_model);
		blob_shadow_view.update(&blob_shadow_model, &scene);

		engine.imgui.set_input_enabled(debug_model.active);
		engine.imgui.set_visible(true);

		let uniforms = build_uniforms(&camera, engine.gfx.aspect());
		uniform_buffer.upload_single(&uniforms);


		let mut view_ctx = views::ViewContext::new(&mut engine);

		view_ctx.gfx.set_clear_color(Color::grey_a(0.1, 0.0));
		view_ctx.gfx.clear(gfx::ClearMode::ALL);

		view_ctx.gfx.bind_uniform_buffer(0, uniform_buffer);

		view_ctx.gfx.bind_framebuffer(test_fbo);
		view_ctx.gfx.clear(gfx::ClearMode::ALL);

		scene_view.draw(&mut view_ctx);

		view_ctx.gfx.bind_framebuffer(test_fbo2);
		view_ctx.gfx.clear(gfx::ClearMode::ALL);

		player_view.draw(&mut view_ctx);
		blob_shadow_view.draw(&mut view_ctx);

		view_ctx.gfx.bind_framebuffer(None);

		

		{
			let _scope = view_ctx.perf.scoped_section("post process");

			let color_0 = test_fbo.color_attachment(0);
			let backbuffer_size = view_ctx.gfx.backbuffer_size();

			let compute_workgroups = (backbuffer_size + Vec2i::splat(15)) / 16;
			let Vec2i{x: compute_w, y: compute_h} = compute_workgroups;

			view_ctx.gfx.bind_image_for_rw(0, color_0);
			view_ctx.gfx.bind_shader(post_effect_compute_shader);
			view_ctx.gfx.dispatch_compute(compute_w as u32, compute_h as u32, 1);

			// Insert barrier because we fetch from color_0 in the next draw call
			view_ctx.gfx.insert_texture_barrier();

			let color_1 = test_fbo2.color_attachment(0);
			let depth_0 = test_fbo.depth_stencil_attachment();
			let depth_1 = test_fbo2.depth_stencil_attachment();

			view_ctx.gfx.bind_texture(0, color_0);
			view_ctx.gfx.bind_texture(1, color_1);
			view_ctx.gfx.bind_texture(2, depth_0);
			view_ctx.gfx.bind_texture(3, depth_1);
			view_ctx.gfx.bind_shader(composite_shader);
			view_ctx.gfx.draw_arrays(gfx::DrawMode::Triangles, 6);
		}

		gbuffer_particles_view.update(&mut view_ctx, test_fbo);
		gbuffer_particles_view.draw(&mut view_ctx);

		view_ctx.gfx.clear(gfx::ClearMode::DEPTH);

		if debug_model.active {
			debug_view.draw(&mut view_ctx, &debug_model);
		}

		engine = next_frame(engine).await;
	}

	Ok(())
}






fn build_uniforms(camera: &model::Camera, aspect: f32) -> shaders::StdUniforms {
	let projection_view = {
		let camera_orientation = Quat::from_pitch(-camera.pitch) * Quat::from_yaw(-camera.yaw);
		let camera_orientation = camera_orientation.to_mat4();

		Mat4::perspective(PI/3.0, aspect, 0.1, 1000.0)
			* camera_orientation
			* Mat4::translate(-camera.position)
	};

	shaders::StdUniforms {
		projection_view,
		projection_view_inverse: projection_view.inverse(),

		ui_projection_view: {
			Mat4::scale(Vec3::new(1.0 / aspect, 1.0, 1.0))
		}
	}
}