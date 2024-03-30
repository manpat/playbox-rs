#![feature(let_chains)]

use toybox::*;

mod audio;
mod sprites;
mod world;
mod toy_draw;
mod game_scene;
mod main_menu;

use audio::MyAudioSystem;
use game_scene::GameScene;
use main_menu::MainMenuScene;


fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_BACKTRACE", "1");
	toybox::run("playbox", App::new)
}




pub enum ActiveScene {
	MainMenu,
	Game,
}

struct App {
	active_scene: ActiveScene,

	main_menu: MainMenuScene,
	game_scene: GameScene,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.show_debug_menu = true;

		dbg!(&ctx.gfx.core.capabilities());
		dbg!(ctx.resource_root_path());

		let audio = MyAudioSystem::start(&mut ctx.audio)?;

		let mut active_scene = ActiveScene::MainMenu;

		if false /*cfg.read_bool("skip-main-menu")*/ {
			active_scene = ActiveScene::Game;
		}

		Ok(App {
			active_scene,
			main_menu: MainMenuScene::new(ctx, audio.clone())?,
			game_scene: GameScene::new(ctx, audio.clone())?,
		})
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		match self.active_scene {
			ActiveScene::MainMenu => {
				if ctx.input.button_just_down(input::Key::Space) {
					self.active_scene = ActiveScene::Game;
				}

				self.main_menu.update(ctx);
			}

			ActiveScene::Game => {
				if ctx.input.button_just_down(input::Key::Escape) {
					self.active_scene = ActiveScene::MainMenu;
				}

				self.game_scene.update(ctx);
				self.game_scene.draw(&mut ctx.gfx);
			}
		}

	}

	fn customise_debug_menu(&mut self, ui: &mut egui::Ui) {
		ui.menu_button("Playbox", |ui| {
			let _ = ui.button("???");
		});
	}
}
