#![feature(array_chunks)]
#![feature(must_not_suspend)]

// Disabled because it doesn't seem to track drops properly
// #![feature(must_not_suspend)]
// #![deny(must_not_suspend)]

use toybox::prelude::*;

mod shaders;
mod executor;
mod intersect;

mod global_controller;

mod platformer3d;
mod balls;

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
	Quit,
}


async fn main_game_loop() -> Result<(), Box<dyn Error>> {
	loop {
		match main_menu().await? {
			MainMenuCommand::PlayPlatformerScene(scene) => {
				platformer3d::load_and_play_scene("assets/scene.toy", scene).await?;
			}

			MainMenuCommand::PlayBalls => {
				balls::play().await?;
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

	// let view_resource_context = engine.gfx.resource_context(&resource_scope_token);


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



