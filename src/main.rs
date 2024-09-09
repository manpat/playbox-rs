#![feature(let_chains)]

pub mod audio;
pub mod menu;
pub mod sprites;
pub mod toy_draw;
pub mod game_scene;
pub mod main_menu;
pub mod glyph_cache;
pub mod message_bus;

pub mod view;
pub mod model;

pub mod console;
pub mod editor;

pub mod prelude {
	pub use toybox::prelude::*;

	pub use crate::audio::MyAudioSystem;
	pub use crate::game_scene::GameScene;
	pub use crate::main_menu::{MainMenuScene, MenuCmd, PauseMenuScene};
	pub use crate::sprites::Sprites;
	pub use crate::toy_draw::ToyRenderer;
	pub use crate::menu;

	pub use crate::model;
	pub use crate::view;

	pub use crate::editor;

	pub use crate::glyph_cache::GlyphCache;

	pub use crate::Context;
	pub use crate::message_bus::{MessageBus, Subscription};
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
	game_scene: Option<GameScene>,

	console: console::Console,

	message_bus: MessageBus,
	menu_cmd_subscription: Subscription<MenuCmd>,

	audio: MyAudioSystem,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		let message_bus = MessageBus::new();
		let audio = MyAudioSystem::start(&mut ctx.audio)?;
		let ctx = &mut Context::new(ctx, &audio, &message_bus);

		let mut active_scene = ActiveScene::MainMenu;
		let mut game_scene = None;

		if true /*ctx.cfg.read_bool("skip-main-menu")*/ {
			active_scene = ActiveScene::Game;
			let world = Self::load_world_or_default(ctx.vfs.resource_root().join("worlds/default.world"));
			game_scene = Some(GameScene::new(ctx, world)?);
		}

		Ok(App {
			active_scene,
			main_menu: MainMenuScene::new(ctx)?,
			pause_menu: PauseMenuScene::new(ctx)?,
			game_scene,

			console: console::Console::new(message_bus.clone()),

			menu_cmd_subscription: message_bus.subscribe(),
			message_bus,

			audio,
		})
	}

	fn load_world_or_default(path: impl AsRef<std::path::Path>) -> model::World {
		let path = path.as_ref();

		match model::World::load(path) {
			Ok(world) => world,
			Err(err) => {
				eprintln!("Failed to load world at '{}', creating empty world. {err}", path.display());
				model::World::new()
			},
		}
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		self.message_bus.garbage_collect();

		self.console.update(ctx);

		if let ActiveScene::Game | ActiveScene::PauseMenu = self.active_scene
			&& self.game_scene.is_none()
		{
			eprintln!("Active scene wants game scene but no game scene is loaded. Transitioning back to main menu");
			self.active_scene = ActiveScene::MainMenu;
		}

		match self.active_scene {
			ActiveScene::MainMenu => {
				self.main_menu.update(&mut Context::new(ctx, &self.audio, &self.message_bus));
			}

			ActiveScene::Game => {
				let game_scene = self.game_scene.as_mut().unwrap();

				if ctx.input.button_just_down(input::keys::Escape) {
					self.active_scene = ActiveScene::PauseMenu;
				}

				if !self.console.is_visible() {
					game_scene.update(&mut Context::new(ctx, &self.audio, &self.message_bus));
				}

				game_scene.draw(&mut ctx.gfx);
			}

			ActiveScene::PauseMenu => {
				// TODO(pat.m): fullscreen quad vignette/transparent backdrop
				let game_scene = self.game_scene.as_mut().unwrap();
				
				self.pause_menu.update(&mut Context::new(ctx, &self.audio, &self.message_bus));
				game_scene.draw(&mut ctx.gfx);
			}
		}

		for menu_msg in self.message_bus.poll_consume(&self.menu_cmd_subscription) {
			match menu_msg {
				MenuCmd::Play(..) => {
					let world = Self::load_world_or_default(ctx.vfs.resource_path("worlds/default.world"));
					let ctx = &mut Context::new(ctx, &self.audio, &self.message_bus);
					self.game_scene = Some(GameScene::new(ctx, world).expect("Failed to initialise GameScene"));
					self.active_scene = ActiveScene::Game;
				}

				MenuCmd::Resume => {
					if self.game_scene.is_some() {
						self.active_scene = ActiveScene::Game;
					}
				}

				MenuCmd::QuitToMain => {
					// TODO(pat.m): confirmation/save
					self.game_scene = None;
					self.active_scene = ActiveScene::MainMenu;
				}

				MenuCmd::QuitToDesktop => {
					// TODO(pat.m): confirmation/save
					ctx.wants_quit = true;
				}

				MenuCmd::Settings => {}
			}
		}
	}

	fn customise_debug_menu(&mut self, ctx: &mut toybox::Context, ui: &mut egui::Ui) {
		match self.active_scene {
			ActiveScene::PauseMenu | ActiveScene::Game => {
				if let Some(game_scene) = &mut self.game_scene {
					game_scene.add_editor_debug_menu(ctx, ui);
				}
			}

			_ => {}
		}
	}
}


pub struct Context<'tb> {
	pub gfx: &'tb mut toybox::gfx::System,
	pub input: &'tb mut toybox::input::System,
	pub egui: &'tb mut toybox::egui::Context,
	pub cfg: &'tb mut toybox::cfg::Config,
	pub vfs: &'tb toybox::vfs::Vfs,

	pub message_bus: &'tb MessageBus,
	pub audio: &'tb MyAudioSystem,
}

impl<'tb> Context<'tb> {
	pub fn new(tb: &'tb mut toybox::Context, audio: &'tb MyAudioSystem, message_bus: &'tb MessageBus) -> Self {
		let toybox::Context { gfx, input, egui, cfg, vfs, .. } = tb;

		Self {gfx, input, egui, cfg, vfs, audio, message_bus}
	}
}

