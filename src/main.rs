#![feature(array_chunks)]
#![feature(portable_simd)]
#![feature(must_not_suspend)]

// Disabled because it doesn't seem to track drops properly
// #![feature(must_not_suspend)]
// #![deny(must_not_suspend)]

use toybox::prelude::*;

mod shaders;
mod executor;
mod intersect;

mod global_controller;
mod vignette;

use executor::{start_loop, next_frame};

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	let mut main_resource_context = engine.gfx.resource_context(None);
	main_resource_context.add_shader_import("global", shaders::GLOBAL_COMMON);

	executor::run_main_loop(&mut engine, main_game_loop())
}





enum MainMenuCommand {
	PlayPlatformerScene(&'static str),
	PlayBalls,
	PlayDungeon,
	MeshBuilderTest,
	SoundTest,
	Quit,
}


async fn main_game_loop() -> Result<(), Box<dyn Error>> {
	loop {
		match main_menu().await? {
			MainMenuCommand::PlayPlatformerScene(scene) => {
				vignette::platformer3d::load_and_play_scene("assets/scene.toy", scene).await?;
			}

			MainMenuCommand::PlayBalls => {
				vignette::balls::play().await?;
			}

			MainMenuCommand::PlayDungeon => {
				vignette::dungeon::play().await?
			}

			MainMenuCommand::MeshBuilderTest => {
				vignette::mesh_builder_test::play().await?
			}

			MainMenuCommand::SoundTest => {
				vignette::sound_test::play().await?
			}

			MainMenuCommand::Quit => return Ok(())
		}
	}
}




async fn main_menu() -> Result<MainMenuCommand, Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = global_controller::GlobalController::new(&mut engine, resource_scope_token.id())?;

	let scene_list: Vec<_> = {
		let scene_data = std::fs::read("assets/scene.toy")?;
		let source_data = toy::load(&scene_data)?;
		source_data.scenes.iter()
			.map(|s| s.name.clone())
			.collect()
	};


	'main: loop {
		global_controller.update(&mut engine);
		if global_controller.should_quit() {
			break 'main
		}

		engine.imgui.set_input_enabled(true);
		engine.imgui.set_visible(true);


		let mut gfx = engine.gfx.draw_context();
		gfx.set_clear_color(Color::grey(0.1));
		gfx.clear(gfx::ClearMode::ALL);

		// TODO(pat.m): this should be automatic - it shouldn't be so easy to accidentally rely on default pipeline states
		// stops accidental sharing of pipeline state between vignettes.
		gfx.set_depth_test(true);
		gfx.set_backface_culling(true);


		let ui = engine.imgui.frame();

		if let Some(_window) = imgui::Window::new("Main Menu")
			.size([300.0, -1.0], imgui::Condition::Once)
			.position([30.0, 30.0], imgui::Condition::Appearing)
			.begin(ui)
		{
			if ui.button("Main Scene") {
				return Ok(MainMenuCommand::PlayPlatformerScene("main"));
			}

			if ui.button("Second Scene") {
				return Ok(MainMenuCommand::PlayPlatformerScene("second"));
			}

			if ui.button("Balls") {
				return Ok(MainMenuCommand::PlayBalls);
			}

			if ui.button("Dungeon") {
				return Ok(MainMenuCommand::PlayDungeon);
			}

			if ui.button("Mesh Builder Test") {
				return Ok(MainMenuCommand::MeshBuilderTest);
			}

			if ui.button("Sound Test") {
				return Ok(MainMenuCommand::SoundTest);
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

		engine = next_frame(engine).await;
	}

	Ok(MainMenuCommand::Quit)
}



