use crate::prelude::*;

pub struct GameScene {
	fog_shader: gfx::ShaderHandle,
	hdr_to_ldr_shader: gfx::ShaderHandle,

	hdr_color_rt: gfx::ImageHandle,
	depth_rt: gfx::ImageHandle,

	ldr_color_image: gfx::ImageHandle,

	// toy_renderer: ToyRenderer,
	sprites: Sprites,

	world_view: view::WorldView,
	hud_view: view::HudView,

	message_bus: MessageBus,

	model: model::Model,

	time: f32,
	height_offset: f32,

	editor_state: editor::State,
	force_game_controls: bool,
}

impl GameScene {
	pub fn new(ctx: &mut Context<'_>, world: model::World) -> anyhow::Result<GameScene> {
		let gfx::System{ resource_manager, .. } = &mut ctx.gfx;

		let rt_fraction = 4;
		let hdr_color_rt = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget("hdr rendertarget", gfx::ImageFormat::hdr_color(), rt_fraction));
		let depth_rt = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget("depthbuffer", gfx::ImageFormat::Depth, rt_fraction));

		let ldr_color_image = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget("ldr color image", gfx::ImageFormat::Srgba8, rt_fraction));

		// let toy_renderer = {
		// 	let project_path = resource_manager.resource_path().join("toys/basic.toy");
		// 	let project_data = std::fs::read(&project_path)?;
		// 	let project = toy::load(&project_data)?;

		// 	let mut toy_renderer = ToyRenderer::new(&core, resource_manager);
		// 	toy_renderer.set_color_target(hdr_color_rt);
		// 	toy_renderer.set_depth_target(depth_rt);
		// 	toy_renderer.update(&core, |builder| {
		// 		builder.set_root_transform(Mat3x4::scale_translate(Vec3::splat(0.2), Vec3::from_y(0.3)));
		// 		builder.add_entities(project.find_scene("main").unwrap());
		// 	});
		// 	toy_renderer
		// };

		let processed_world = model::ProcessedWorld::new(&world, &ctx.message_bus);

		Ok(GameScene {
			fog_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/fog.cs.glsl")?),
			hdr_to_ldr_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/hdr_to_ldr.cs.glsl")?),

			hdr_color_rt,
			depth_rt,

			ldr_color_image,

			// toy_renderer,
			sprites: Sprites::new(&mut ctx.gfx)?,
			world_view: view::WorldView::new(&mut ctx.gfx, &world, &processed_world, ctx.message_bus.clone())?,
			hud_view: view::HudView::new(&mut ctx.gfx, ctx.message_bus.clone())?,

			message_bus: ctx.message_bus.clone(),

			model: model::Model {
				player: model::Player {
					placement: world.player_spawn,
					pitch: 0.0,

					free_pos: Vec3::zero(),
					free_cam: false,

					hack_height_change: None,
				},

				interactions: model::Interactions::new(ctx.message_bus),
				environment: model::EnvironmentModel::new(&world, ctx.message_bus),
				hud: model::HudModel::new(ctx.message_bus),
				processed_world,

				progress: model::ProgressModel::default(),

				world,
			},

			time: 0.0,
			height_offset: 0.0,

			editor_state: editor::State::new(ctx.message_bus),
			force_game_controls: false,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		if ctx.input.button_just_down(input::keys::F1) {
			self.force_game_controls = !self.force_game_controls;
		}

		ctx.input.set_capture_mouse(!ctx.show_editor || self.force_game_controls);

		if ctx.show_editor {
			editor::draw_world_editor(&ctx.egui, &mut self.editor_state, &self.model, &self.message_bus);
			editor::handle_editor_cmds(&mut self.editor_state, &mut self.model, &self.message_bus);
		}

		let model::Model { processed_world, world, player, progress, interactions, environment, hud, .. } = &mut self.model;

		processed_world.update(&world, &progress, &self.message_bus);

		if !ctx.show_editor || self.force_game_controls {
			player.handle_input(ctx, &world, &processed_world, &hud);
			interactions.update(&player, &world, &processed_world, &self.message_bus);
		}

		hud.update(&self.message_bus);

		environment.update(&world, &self.message_bus);

		self.sprites.set_billboard_orientation(Vec3::from_y(1.0), Vec3::from_y_angle(player.placement.yaw));
	}

	pub fn draw(&mut self, gfx: &mut gfx::System) {
		self.time += 1.0/60.0;

		let player = &self.model.player;

		if let Some(height_change) = player.hack_height_change {
			self.height_offset += height_change;
		}

		if self.height_offset.abs() > 0.02 {
			self.height_offset -= self.height_offset.signum() * (self.height_offset.abs()/5.0).max(0.02);
		} else {
			self.height_offset = 0.0;
		}


		let eye_position = player.placement.position.to_x0y() + Vec3::from_y(model::PLAYER_HEIGHT - self.height_offset) + player.free_pos;

		let aspect = gfx.backbuffer_aspect();
		let projection = Mat4::perspective(80.0f32.to_radians(), aspect, 0.01, 100.0);
		let projection_view = projection
			* Mat4::rotate_x(player.pitch)
			* Mat4::rotate_y(player.placement.yaw)
			* Mat4::translate(-eye_position);

		let inverse_projection = projection.inverse();

		gfx.frame_encoder.backbuffer_color(self.model.world.fog.color);
		gfx.frame_encoder.bind_global_ubo(0, &[projection_view, inverse_projection]);
		gfx.frame_encoder.bind_global_sampled_image(0, gfx::BlankImage::White, gfx::CommonSampler::Nearest);

		let mut main_group = gfx.frame_encoder.command_group(gfx::FrameStage::Main);
		main_group.bind_rendertargets(&[self.hdr_color_rt, self.depth_rt]);

		self.world_view.draw(gfx, &self.model.world, &self.model.processed_world, player.placement);
		self.hud_view.draw(gfx, &self.model);

		// self.toy_renderer.draw(gfx);
		self.sprites.draw(gfx);

		self.dispatch_postprocess(gfx);
	}

	fn dispatch_postprocess(&self, gfx: &mut gfx::System) {
		let gfx::System { frame_encoder, .. } = gfx;

		let mut group = frame_encoder.command_group(gfx::FrameStage::Postprocess);

		#[repr(C)]
		#[derive(Copy, Clone)]
		struct FogParameters {
			fog_color: Color,
			fog_start: f32,
			fog_distance: f32,
			fog_emission: f32,
			fog_transparency: f32,
		}

		group.compute(self.fog_shader)
			.image_rw(0, self.hdr_color_rt)
			.sampled_image(1, self.depth_rt, gfx::CommonSampler::Nearest)
			.ubo(1, &[FogParameters {
				fog_color: self.model.environment.fog.color,
				fog_start: self.model.environment.fog.start,
				fog_distance: self.model.environment.fog.distance,
				fog_emission: self.model.environment.fog.emission,
				fog_transparency: self.model.environment.fog.transparency,
			}])
			.groups_from_image_size(self.hdr_color_rt);

		// TODO(pat.m): bloom

		// Tonemap, gamma correct and dither.
		group.compute(self.hdr_to_ldr_shader)
			.image(0, self.hdr_color_rt)
			.image_rw(1, self.ldr_color_image)
			.ssbo(0, &[self.time])
			.groups_from_image_size(self.hdr_color_rt);

		// Scale and blit to screen
		group.draw_fullscreen(None)
			.sampled_image(0, self.ldr_color_image, gfx::CommonSampler::Nearest);
	}
}




impl GameScene {
	pub fn add_editor_debug_menu(&mut self, ctx: &mut toybox::Context, ui: &mut egui::Ui) {
		ui.menu_button("Editor", |ui| {
			if ui.button("New World").clicked() {
				self.model.world = model::World::new();
				self.message_bus.emit(model::WorldChangedEvent);

				// TODO(pat.m): switch to Game state

				ui.close_menu();
			}

			if ui.button("Load World").clicked() {
				let default_world_path = "worlds/default.world";

				match ctx.vfs.load_json_resource::<model::World>(default_world_path) {
					Ok(new_world) => {
						self.model.world = new_world;
						self.message_bus.emit(model::WorldChangedEvent);
						// TODO(pat.m): switch to Game state
					}

					Err(error) => {
						log::error!("Failed to load world '{default_world_path}': {error}");
					}
				}

				ui.close_menu();
			}

			if ui.button("Save World").clicked() {
				let default_world_path = "worlds/default.world";

				if let Err(error) = ctx.vfs.save_json_resource(default_world_path, &self.model.world) {
					log::error!("Failed to save world to '{default_world_path}': {error}");
				}

				ui.close_menu();
			}

			ui.separator();

			if ui.button("Set Player Spawn").clicked() {
				self.message_bus.emit(editor::EditorWorldEditCmd::SetPlayerSpawn);
				ui.close_menu();
			}
		});
	}
}