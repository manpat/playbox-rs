#![feature(array_chunks)]

// Disabled because it doesn't seem to track drops properly
// #![feature(must_not_suspend)]
// #![deny(must_not_suspend)]

use toybox::prelude::*;

mod views;
mod model;
mod controller;
mod shaders;
mod executor;

mod intersect;

use executor::NextFrame;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	let mut main_resource_context = engine.gfx.resource_context(None);
	main_resource_context.add_shader_import("global", shaders::GLOBAL_COMMON);

	executor::run_main_loop(&mut engine, main_game_loop())
}




#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
	projection_view: Mat4,
	projection_view_inverse: Mat4,
	ui_projection_view: Mat4,
	// NOTE: align to Vec4s
}


fn build_uniforms(camera: &model::Camera, aspect: f32) -> Uniforms {
	let projection_view = {
		let camera_orientation = Quat::from_pitch(-camera.pitch) * Quat::from_yaw(-camera.yaw);
		let camera_orientation = camera_orientation.to_mat4();

		Mat4::perspective(PI/3.0, aspect, 0.1, 1000.0)
			* camera_orientation
			* Mat4::translate(-camera.position)
	};

	Uniforms {
		projection_view,
		projection_view_inverse: projection_view.inverse(),

		ui_projection_view: {
			Mat4::scale(Vec3::new(1.0 / aspect, 1.0, 1.0))
		}
	}
}



enum MainMenuCommand {
	PlayScene(&'static str),
	Quit,
}


async fn main_game_loop() -> Result<(), Box<dyn Error>> {
	loop {
		match main_menu().await? {
			MainMenuCommand::PlayScene(scene) => {
				load_and_play_scene("assets/scene.toy", scene).await?;
			}

			MainMenuCommand::Quit => return Ok(())
		}
	}
}




async fn main_menu() -> Result<MainMenuCommand, Box<dyn Error>> {
	let mut engine = NextFrame.await;

	let mut global_controller = controller::GlobalController::new(&mut engine)?;

	let scene_list: Vec<_> = {
		let scene_data = std::fs::read("assets/scene.toy")?;
		let source_data = toy::load(&scene_data)?;
		source_data.scenes.iter()
			.map(|s| s.name.clone())
			.collect()
	};

	let view_resource_scope_token = engine.gfx.new_resource_scope();
	let mut view_resource_context = engine.gfx.resource_context(&view_resource_scope_token);

	drop(engine);


	'main: loop {
		let mut engine = NextFrame.await;

		global_controller.update(&mut engine);
		if global_controller.should_quit() {
			break 'main
		}

		engine.imgui.set_input_enabled(true);
		engine.imgui.set_visible(true);


		let mut view_ctx = views::ViewContext::new(&mut engine);

		view_ctx.gfx.set_clear_color(Color::grey(0.1));
		view_ctx.gfx.clear(gfx::ClearMode::ALL);


		let ui = engine.imgui.frame();

		if let Some(_window) = imgui::Window::new("Main Menu")
			.size([300.0, -1.0], imgui::Condition::Once)
			.position([30.0, 30.0], imgui::Condition::Appearing)
			.begin(ui)
		{
			if ui.button("Main Scene") {
				return Ok(MainMenuCommand::PlayScene("main"));
			}

			if ui.button("Second Scene") {
				return Ok(MainMenuCommand::PlayScene("second"));
			}

			if ui.button("Quit") {
				break 'main
			}
		}

		if let Some(_window) = imgui::Window::new("Scene List")
			.size([300.0, -1.0], imgui::Condition::Once)
			.position([350.0, 30.0], imgui::Condition::Appearing)
			.begin(ui)
		{
			if let Some(_list) = imgui::ListBox::new("scene list")
				.size([-1.0, 0.0])
				.begin(ui)
			{
				for scene in scene_list.iter() {
					ui.text(format!("{scene}"));
				}
			}
		}
	}

	Ok(MainMenuCommand::Quit)
}



async fn load_and_play_scene(project_path: impl AsRef<std::path::Path>, scene_name: impl Into<String>) -> Result<(), Box<dyn Error>> {
	let mut engine = NextFrame.await;

	let mut player = model::Player::new();
	let mut camera = model::Camera::new();
	let mut debug_model = model::Debug::new();
	let mut scene = model::Scene::new(project_path, scene_name)?;

	let mut blob_shadow_model = model::BlobShadowModel::new();


	let mut global_controller = controller::GlobalController::new(&mut engine)?;
	let mut player_controller = controller::PlayerController::new(&mut engine);
	let mut camera_controller = controller::CameraController::new(&mut engine);
	let mut debug_camera_controller = controller::DebugCameraController::new(&mut engine);
	let mut gem_controller = controller::GemController::new(&mut engine)?;
	let mut audio_test_controller = controller::AudioTestController::new(&mut engine, &scene);
	let debug_controller = controller::DebugController::new(&mut engine);


	let view_resource_scope_token = engine.gfx.new_resource_scope();
	let mut view_resource_context = engine.gfx.resource_context(&view_resource_scope_token);
	let mut uniform_buffer = view_resource_context.new_buffer(gfx::BufferUsage::Stream);

	let mut player_view = views::PlayerView::new(&mut view_resource_context)?;
	let mut debug_view = views::DebugView::new(&mut view_resource_context, &scene)?;
	let mut scene_view = views::SceneView::new(&mut view_resource_context, &scene)?;
	let mut blob_shadow_view = views::BlobShadowView::new(&mut view_resource_context)?;
	let mut mesh_builder_test_view = views::MeshBuilderTestView::new(&mut view_resource_context)?;
	let mut _gbuffer_particles_view = views::GBufferParticlesView::new(&mut view_resource_context)?;

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

	drop(engine);


	'main: loop {
		let mut engine = NextFrame.await;

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
		mesh_builder_test_view.update();

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
		mesh_builder_test_view.draw(&mut view_ctx);

		view_ctx.gfx.bind_framebuffer(None);

		

		{
			let _scope = view_ctx.perf.scoped_section("post process");

			let resources = view_ctx.resources;
			let fbo_0 = resources.get(test_fbo);
			let color_0 = fbo_0.color_attachment(0).unwrap();
			let color_0_size = resources.get(color_0).size();

			let compute_workgroups = (color_0_size + Vec2i::splat(15)) / 16;
			let Vec2i{x: compute_w, y: compute_h} = compute_workgroups;

			view_ctx.gfx.bind_image_for_rw(0, color_0);
			view_ctx.gfx.bind_shader(post_effect_compute_shader);
			view_ctx.gfx.dispatch_compute(compute_w as u32, compute_h as u32, 1);

			unsafe {
				gfx::raw::MemoryBarrier(gfx::raw::TEXTURE_FETCH_BARRIER_BIT);
			}

			let color_1 = resources.get(test_fbo2).color_attachment(0).unwrap();
			let depth_0 = resources.get(test_fbo).depth_stencil_attachment().unwrap();
			let depth_1 = resources.get(test_fbo2).depth_stencil_attachment().unwrap();

			view_ctx.gfx.bind_texture(0, color_0);
			view_ctx.gfx.bind_texture(1, color_1);
			view_ctx.gfx.bind_texture(2, depth_0);
			view_ctx.gfx.bind_texture(3, depth_1);
			view_ctx.gfx.bind_shader(composite_shader);
			view_ctx.gfx.draw_arrays(gfx::DrawMode::Triangles, 6);
		}

		// gbuffer_particles_view.update(&mut view_ctx, test_fbo);
		// gbuffer_particles_view.draw(&mut view_ctx);

		view_ctx.gfx.clear(gfx::ClearMode::DEPTH);

		if debug_model.active {
			debug_view.draw(&mut view_ctx, &debug_model);
			mesh_builder_test_view.draw_2d(&mut view_ctx);
		}
	}

	Ok(())
}


