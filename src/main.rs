#![feature(let_chains)]

use toybox::*;

pub mod audio;
pub mod sprites;
pub mod world;
pub mod toy_draw;
pub mod game_scene;
pub mod main_menu;

pub mod prelude {
	pub use toybox::prelude::*;

	pub use crate::audio::MyAudioSystem;
	pub use crate::game_scene::GameScene;
	pub use crate::main_menu::MainMenuScene;
	pub use crate::sprites::Sprites;
	pub use crate::toy_draw::ToyRenderer;
	pub use crate::world;
}

use prelude::*;

fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_BACKTRACE", "1");
	toybox::run("playbox", App::new)
}




pub enum ActiveScene {
	MainMenu,

	Game,
	PauseMenu,
}

struct App {
	active_scene: ActiveScene,

	main_menu: MainMenuScene,
	game_scene: GameScene,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.show_debug_menu = cfg!(debug_assertions);

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

				ctx.input.set_capture_mouse(false);

				self.main_menu.update(ctx);
			}

			ActiveScene::Game => {
				if ctx.input.button_just_down(input::Key::Escape) {
					self.active_scene = ActiveScene::PauseMenu;
				}

				self.game_scene.update(ctx);
				self.game_scene.draw(&mut ctx.gfx);
			}

			ActiveScene::PauseMenu => {
				if ctx.input.button_just_down(input::Key::Escape) {
					self.active_scene = ActiveScene::Game;
				}

				ctx.input.set_capture_mouse(false);

				// TODO(pat.m): menu builder
				// TODO(pat.m): fullscreen quad vignette/transparent backdrop
				ctx.gfx.frame_encoder.command_group(gfx::FrameStage::Ui(0))
					.draw_fullscreen(None)
					.sampled_image(0, ctx.gfx.resource_manager.blank_black_image, ctx.gfx.resource_manager.nearest_sampler);

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
