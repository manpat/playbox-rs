#![feature(array_chunks)]

// Disabled because it doesn't seem to track drops properly
// #![feature(must_not_suspend)]
// #![deny(must_not_suspend)]

use toybox::prelude::*;

mod shaders;
mod executor;
mod intersect;

mod platformer3d;

use executor::NextFrame;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	let mut main_resource_context = engine.gfx.resource_context(None);
	main_resource_context.add_shader_import("global", shaders::GLOBAL_COMMON);

	executor::run_main_loop(&mut engine, main_game_loop())
}





enum MainMenuCommand {
	PlayPlatformerScene(&'static str),
	Quit,
}


async fn main_game_loop() -> Result<(), Box<dyn Error>> {
	loop {
		match main_menu().await? {
			MainMenuCommand::PlayPlatformerScene(scene) => {
				platformer3d::load_and_play_scene("assets/scene.toy", scene).await?;
			}

			MainMenuCommand::Quit => return Ok(())
		}
	}
}




async fn main_menu() -> Result<MainMenuCommand, Box<dyn Error>> {
	let mut engine = NextFrame.await;

	let mut global_controller = platformer3d::controller::GlobalController::new(&mut engine)?;

	let scene_list: Vec<_> = {
		let scene_data = std::fs::read("assets/scene.toy")?;
		let source_data = toy::load(&scene_data)?;
		source_data.scenes.iter()
			.map(|s| s.name.clone())
			.collect()
	};

	let view_resource_scope_token = engine.gfx.new_resource_scope();
	let _view_resource_context = engine.gfx.resource_context(&view_resource_scope_token);

	drop(engine);


	'main: loop {
		let mut engine = NextFrame.await;

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



