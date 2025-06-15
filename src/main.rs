#![feature(let_chains)]

pub mod aabb2_ext;

pub mod audio;
pub mod ui;
pub mod sprites;
pub mod toy_draw;
pub mod game_scene;
pub mod main_menu;
pub mod glyph_cache;

pub mod view;
pub mod model;

pub mod console;
pub mod editor;

pub mod prelude {
	pub use toybox::prelude::*;
	pub use toybox::bus::{MessageBus, Subscription};

	pub use crate::aabb2_ext::*;

	pub use crate::audio::MyAudioSystem;
	pub use crate::game_scene::GameScene;
	pub use crate::main_menu::{MainMenuScene, MenuCmd, PauseMenuScene};
	pub use crate::sprites::Sprites;
	pub use crate::toy_draw::ToyRenderer;
	pub use crate::ui::{self, UiLayout};

	pub use crate::model;
	pub use crate::view;

	pub use crate::console::Console;
	pub use crate::editor;

	pub use crate::glyph_cache::GlyphCache;

	pub use crate::Context;

	pub use std::collections::HashMap;
	pub use std::borrow::Cow;

	pub use slotmap::SlotMap;
	pub use smallvec::SmallVec;
}

use std::time::{Instant, Duration};

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

	menu_cmd_subscription: Subscription<MenuCmd>,

	shared: AppShared,
	frame_start: Instant,
}


struct AppShared {
	console: console::Console,
	ui_shared: ui::UiShared,

	audio: MyAudioSystem,
	delta_time: f32,
}


impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		let menu_cmd_subscription = ctx.bus.subscribe();

		let audio = MyAudioSystem::start(&mut ctx.audio)?;
		let ui_shared = ui::UiShared::new(&mut ctx.gfx)?;
		let mut console = console::Console::new();

		register_commands(&mut console);

		let mut shared = AppShared {
			console,
			ui_shared,
			audio,
			delta_time: 1.0/60.0,
		};

		let ctx = &mut Context::new(ctx, &mut shared);

		let mut active_scene = ActiveScene::MainMenu;
		let mut game_scene = None;

		if let Some(world_name) = ctx.cfg.get_string("start.load-world") {
			active_scene = ActiveScene::Game;
			let world = Self::load_world_or_default(&ctx.vfs, world_name);
			game_scene = Some(GameScene::new(ctx, world)?);
		}

		Ok(App {
			active_scene,
			main_menu: MainMenuScene::new(ctx)?,
			pause_menu: PauseMenuScene::new(ctx)?,
			game_scene,

			menu_cmd_subscription,

			shared,
			frame_start: Instant::now(),
		})
	}

	fn load_world_or_default(vfs: &vfs::Vfs, world_name: impl AsRef<str>) -> model::World {
		let world_name = world_name.as_ref();
		let path = format!("worlds/{world_name}.world");

		match vfs.load_json_resource(path) {
			Ok(world) => world,
			Err(err) => {
				log::error!("Failed to load world '{world_name}', creating empty world. {err}");
				model::World::new()
			},
		}
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		let now = Instant::now();
		let delta = now.duration_since(self.frame_start);
		self.frame_start = now;

		self.shared.delta_time = delta.as_secs_f32();

		self.shared.console.update(ctx);

		if let ActiveScene::Game | ActiveScene::PauseMenu = self.active_scene
			&& self.game_scene.is_none()
		{
			log::error!("Active scene wants game scene but no game scene is loaded. Transitioning back to main menu");
			self.active_scene = ActiveScene::MainMenu;
		}

		match self.active_scene {
			ActiveScene::MainMenu => {
				self.main_menu.update(&mut Context::new(ctx, &mut self.shared));
			}

			ActiveScene::Game => {
				let game_scene = self.game_scene.as_mut().unwrap();

				if ctx.input.button_just_down(input::keys::Escape) {
					self.active_scene = ActiveScene::PauseMenu;
				}

				let mut ctx = Context::new(ctx, &mut self.shared);
				game_scene.update(&mut ctx);
				game_scene.draw(&mut ctx);
			}

			ActiveScene::PauseMenu => {
				let game_scene = self.game_scene.as_mut().unwrap();

				let mut ctx = Context::new(ctx, &mut self.shared);
				self.pause_menu.update(&mut ctx);
				game_scene.draw(&mut ctx);
			}
		}

		for menu_msg in ctx.bus.poll_consume(&self.menu_cmd_subscription) {
			match menu_msg {
				MenuCmd::Play(world_name) => {
					let world = Self::load_world_or_default(&ctx.vfs, world_name);
					let ctx = &mut Context::new(ctx, &mut self.shared);

					// Reuse scene if we can, to avoid reloading common stuff
					if let Some(game_scene) = &mut self.game_scene {
						game_scene.switch_world(ctx, world)
					} else {
						self.game_scene = Some(GameScene::new(ctx, world).expect("Failed to initialise GameScene"));
					}

					self.active_scene = ActiveScene::Game;
				}

				MenuCmd::PlayGeneratedWorld => {
					let world = model::world::generate();
					let ctx = &mut Context::new(ctx, &mut self.shared);

					// Reuse scene if we can, to avoid reloading common stuff
					if let Some(game_scene) = &mut self.game_scene {
						game_scene.switch_world(ctx, world)
					} else {
						self.game_scene = Some(GameScene::new(ctx, world).expect("Failed to initialise GameScene"));
					}

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

		self.shared.ui_shared.glyph_atlas.get_mut().update_atlas(&mut ctx.gfx);
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
	pub bus: &'tb toybox::bus::MessageBus,

	pub audio: &'tb MyAudioSystem,
	pub console: &'tb mut Console,
	pub ui_shared: &'tb mut ui::UiShared,

	pub delta_time: f32,
	pub show_editor: bool,
}

impl<'tb> Context<'tb> {
	fn new(tb: &'tb mut toybox::Context, shared: &'tb mut AppShared) -> Self {
		let toybox::Context { gfx, input, egui, cfg, vfs, bus, show_debug_menu, .. } = tb;
		let AppShared { audio, console, ui_shared, delta_time } = shared;
		let show_editor = *show_debug_menu;
		let delta_time = *delta_time;

		Self {gfx, input, egui, cfg, vfs, bus, audio, ui_shared, console, delta_time, show_editor}
	}
}



fn register_commands(console: &mut Console) {
	console.register_command("quit", |ctx, _| ctx.bus.emit(MenuCmd::QuitToDesktop));
	console.register_command("load", |ctx, world_name| {
		if world_name.is_empty() {
			log::error!("'load' requires world name argument");
			return;
		}

		ctx.bus.emit(MenuCmd::Play(world_name.into()));
	});

	console.register_command("gen", |ctx, _| ctx.bus.emit(MenuCmd::PlayGeneratedWorld));
}