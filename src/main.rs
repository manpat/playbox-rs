#![feature(let_chains)]

use toybox::*;

pub mod audio;
pub mod menu;
pub mod sprites;
pub mod world;
pub mod toy_draw;
pub mod game_scene;
pub mod main_menu;
pub mod glyph_cache;

pub mod editor;

pub mod prelude {
	pub use toybox::prelude::*;

	pub use crate::audio::MyAudioSystem;
	pub use crate::game_scene::GameScene;
	pub use crate::main_menu::{MainMenuScene, MenuCmd, PauseMenuScene};
	pub use crate::sprites::Sprites;
	pub use crate::toy_draw::ToyRenderer;
	pub use crate::world;
	pub use crate::menu;

	pub use crate::editor;

	pub use crate::glyph_cache::GlyphCache;

	pub use crate::Context;
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
	pause_menu: PauseMenuScene,
	game_scene: GameScene,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.show_debug_menu = cfg!(debug_assertions);

		dbg!(&ctx.gfx.core.capabilities());
		dbg!(ctx.resource_root_path());

		let audio = MyAudioSystem::start(&mut ctx.audio)?;

		let mut active_scene = ActiveScene::MainMenu;

		if false /*ctx.cfg.read_bool("skip-main-menu")*/ {
			active_scene = ActiveScene::Game;
		}

		let ctx = &mut Context::new(ctx);

		Ok(App {
			active_scene,
			main_menu: MainMenuScene::new(ctx, audio.clone())?,
			pause_menu: PauseMenuScene::new(ctx)?,
			game_scene: GameScene::new(ctx, audio.clone())?,
		})
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		match self.active_scene {
			ActiveScene::MainMenu => {
				match self.main_menu.update(&mut Context::new(ctx)) {
					Some(MenuCmd::Play) => {
						self.active_scene = ActiveScene::Game;
					}

					Some(MenuCmd::Settings) => {}

					Some(MenuCmd::Quit) => {
						ctx.wants_quit = true;
					}

					_ => {}
				}
			}

			ActiveScene::Game => {
				if ctx.input.button_just_down(input::Key::Escape) {
					self.active_scene = ActiveScene::PauseMenu;
				}

				self.game_scene.update(&mut Context::new(ctx));
				self.game_scene.draw(&mut ctx.gfx);
			}

			ActiveScene::PauseMenu => {
				// TODO(pat.m): fullscreen quad vignette/transparent backdrop
				
				match self.pause_menu.update(&mut Context::new(ctx)) {
					Some(MenuCmd::Play) => {
						self.active_scene = ActiveScene::Game;
					}

					Some(MenuCmd::ReturnToMain) => {
						self.active_scene = ActiveScene::MainMenu;
					}

					Some(MenuCmd::Quit) => {
						ctx.wants_quit = true;
					}

					_ => {}
				}

				self.game_scene.draw(&mut ctx.gfx);
			}
		}
	}

	fn customise_debug_menu(&mut self, ui: &mut egui::Ui) {
		match self.active_scene {
			ActiveScene::PauseMenu | ActiveScene::Game => {
				self.game_scene.add_editor_debug_menu(ui);
			}

			_ => {}
		}
	}
}


pub struct Context<'tb> {
	pub gfx: &'tb mut toybox::gfx::System,
	pub audio: &'tb mut toybox::audio::System,
	pub input: &'tb mut toybox::input::System,
	pub egui: &'tb mut toybox::egui::Context,
	pub cfg: &'tb mut toybox::cfg::Config,
}

impl<'tb> Context<'tb> {
	pub fn new(tb: &'tb mut toybox::Context) -> Self {
		let toybox::Context { gfx, audio, input, egui, cfg, .. } = tb;

		Self {gfx, audio, input, egui, cfg}
	}
}