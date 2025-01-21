use crate::prelude::*;

pub struct GameScene {
	fog_shader: gfx::ShaderHandle,
	hdr_to_ldr_shader: gfx::ShaderHandle,

	downsample_shader: gfx::ShaderHandle,
	upsample_shader: gfx::ShaderHandle,
	bloom_shader: gfx::ShaderHandle,

	downsample_chain: Vec<gfx::ImageHandle>,
	upsample_chain: Vec<gfx::ImageHandle>,

	hdr_color_rt: gfx::ImageHandle,
	depth_rt: gfx::ImageHandle,

	ldr_color_image: gfx::ImageHandle,

	// toy_renderer: ToyRenderer,
	// sprites: Sprites,

	world_view: view::WorldView,
	hud_view: view::HudView,

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
		let hdr_color_rt = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget("hdr rendertarget", gfx::ImageFormat::rgba16f(), rt_fraction));
		let depth_rt = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget("depthbuffer", gfx::ImageFormat::Depth, rt_fraction));

		let ldr_color_image = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget("ldr color image", gfx::ImageFormat::Srgba8, rt_fraction));

		let mut downsample_chain = Vec::new();
		let mut upsample_chain = Vec::new();

		let num_mips = 5;

		for mip in 0..num_mips + 1 {
			let image = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget(format!("downsample mip {mip}"), gfx::ImageFormat::rgba16f(), rt_fraction << (mip + 1)));
			downsample_chain.push(image);
		}

		for mip in 0..num_mips {
			let image = resource_manager.request(gfx::CreateImageRequest::fractional_rendertarget(format!("upsample mip {mip}"), gfx::ImageFormat::rgba16f(), rt_fraction << mip));
			upsample_chain.push(image);
		}

		// let toy_renderer = {
		// 	let project_path = resource_manager.resource_path("toys/basic.toy")?;
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

		let processed_world = model::ProcessedWorld::new(&world, &ctx.bus);

		Ok(GameScene {
			fog_shader: resource_manager.load_compute_shader("shaders/fog.cs.glsl"),
			hdr_to_ldr_shader: resource_manager.load_compute_shader("shaders/hdr_to_ldr.cs.glsl"),

			downsample_shader: resource_manager.load_compute_shader("shaders/downsample.cs.glsl"),
			upsample_shader: resource_manager.load_compute_shader("shaders/upsample.cs.glsl"),
			bloom_shader: resource_manager.load_compute_shader("shaders/bloom.cs.glsl"),

			hdr_color_rt,
			depth_rt,

			downsample_chain,
			upsample_chain,

			ldr_color_image,

			// toy_renderer,
			// sprites: Sprites::new(&mut ctx.gfx)?,
			world_view: view::WorldView::new(&mut ctx.gfx, &world, &processed_world, ctx.bus.clone())?,
			hud_view: view::HudView::new(&mut ctx.gfx, ctx.bus.clone())?,

			model: model::Model {
				player: model::Player {
					placement: processed_world.to_processed_placement(world.player_spawn),
					pitch: 0.0,

					step_accumulator: 0.0,

					blood: 100,
					salt: 100,

					free_pos: Vec3::zero(),
					free_cam: false,

					hack_height_change: None,
				},

				interactions: model::Interactions::new(ctx.bus),
				environment: model::EnvironmentModel::new(&world, ctx.bus),
				hud: model::HudModel::new(ctx.bus),
				processed_world,

				progress: model::ProgressModel::default(),

				world,
			},

			time: 0.0,
			height_offset: 0.0,

			editor_state: editor::State::new(ctx.bus),
			force_game_controls: false,
		})
	}

	pub fn switch_world(&mut self, ctx: &mut Context<'_>, new_world: model::World) {
		self.model.player.placement = self.model.processed_world.to_processed_placement(new_world.player_spawn);
		self.model.world = new_world;
		ctx.bus.emit(model::WorldChangedEvent);

		self.editor_state.reset();
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		if ctx.input.button_just_down(input::keys::F2) {
			self.force_game_controls = !self.force_game_controls;
		}

		ctx.input.set_capture_mouse(!ctx.show_editor || self.force_game_controls);

		if ctx.show_editor {
			editor::draw_world_editor(&ctx.egui, &mut self.editor_state, &self.model, ctx.bus);
			editor::handle_editor_cmds(&mut self.editor_state, &mut self.model, ctx.bus);
		}

		if let Err(err) = self.handle_console(ctx) {
			log::error!("{err:?}");
		}

		let model::Model { processed_world, world, player, progress, interactions, environment, hud, .. } = &mut self.model;

		processed_world.update(&world, &progress, ctx.bus);

		// TODO(pat.m): needs to happen somewhere else, but has to happen after processed world update
		{
			// Make sure player doesn't suddenly end up in a room that no longer exists.
			if !player.placement.room_id.is_valid(processed_world.geometry()) {
				player.placement = processed_world.to_processed_placement(world.player_spawn);
			}
		}

		if !ctx.show_editor && !ctx.console.is_visible() || self.force_game_controls {
			player.handle_input(ctx, &processed_world, &hud);
			interactions.update(&player, &world, &processed_world, ctx.bus);
		}

		hud.update(ctx.bus);

		environment.update(&world, ctx.bus);

		// self.sprites.set_billboard_orientation(Vec3::from_y(1.0), Vec3::from_y_angle(player.placement.yaw));
	}

	pub fn draw(&mut self, ctx: &mut Context<'_>) {
		let Context{gfx, ui_shared, ..} = ctx;

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
		self.hud_view.draw(gfx, ui_shared, &self.model);

		// self.toy_renderer.draw(gfx);
		// self.sprites.draw(gfx);

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

		group.debug_marker("Fog");

		// Apply fog
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


		// Apply bloom
		{
			let mut source_mip = self.hdr_color_rt;

			group.debug_marker("Downsample");
			for &target_mip in self.downsample_chain.iter() {
				group.compute(self.downsample_shader)
					.sampled_image(0, source_mip, gfx::CommonSampler::Linear)
					.image_rw(1, target_mip)
					.groups_from_image_size(target_mip);

				source_mip = target_mip;
			}

			group.debug_marker("Upsample");
			for (&target_mip, &downsample_mip) in self.upsample_chain.iter().zip(&self.downsample_chain).rev() {
				group.compute(self.upsample_shader)
					.sampled_image(0, source_mip, gfx::CommonSampler::Linear)
					.sampled_image(2, downsample_mip, gfx::CommonSampler::Linear)
					.image_rw(1, target_mip)
					.groups_from_image_size(target_mip);

				source_mip = target_mip;
			}

			group.debug_marker("Composite");
			group.compute(self.bloom_shader)
				.sampled_image(0, source_mip, gfx::CommonSampler::Linear)
				.image_rw(1, self.hdr_color_rt)
				.groups_from_image_size(self.hdr_color_rt);
		}



		#[repr(C)]
		#[derive(Copy, Clone)]
		struct ToneMapParameters {
			dither_time: f32,
			dither_quantise: f32,

			tonemap_contrast: f32,
			tonemap_exposure: f32,

			tonemap_algorithm: ToneMapAlgorithm,
		}

		#[allow(dead_code)]
		#[repr(u32)]
		#[derive(Copy, Clone)]
		enum ToneMapAlgorithm {
			None,
			Reinhardt,
			AcesFilmic,
		}

		group.debug_marker("Tonemap");

		// Tonemap, gamma correct and dither.
		group.compute(self.hdr_to_ldr_shader)
			.image(0, self.hdr_color_rt)
			.image_rw(1, self.ldr_color_image)
			.ssbo(0, &[ToneMapParameters {
				dither_time: self.time,
				// dither_quantise: 256.0,
				dither_quantise: 128.0,

				tonemap_contrast: 2.0,
				tonemap_exposure: 1.0,

				tonemap_algorithm: ToneMapAlgorithm::AcesFilmic,
			}])
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
				// ctx.bus.emit(editor::EditorModalCmd::NewWorld);

				// if changes made to current world, check save

				// TODO(pat.m): this is jank as hell. the model really needs to be split up so the source data
				// can just be replaced wholesale
				self.model.world = model::World::new();
				self.model.player.placement = self.model.world.player_spawn;
				ctx.bus.emit(model::WorldChangedEvent);

				ui.close_menu();
			}

			if ui.button("Load World").clicked() {
				// ctx.bus.emit(editor::EditorModalCmd::LoadWorld);

				ctx.bus.emit(MenuCmd::Play("default".into()));
				ui.close_menu();
			}

			if ui.button("Save World").clicked() {
				// ctx.bus.emit(editor::EditorModalCmd::SaveWorld);

				let default_world_path = "worlds/default.world";

				if let Err(error) = ctx.vfs.save_json_resource(default_world_path, &self.model.world) {
					log::error!("Failed to save world to '{default_world_path}': {error}");
				}

				ui.close_menu();
			}
		});
	}

	fn handle_console(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
		if let Some(world_name) = ctx.console.command("load") {
			if world_name.is_empty() {
				anyhow::bail!("'load' requires world name argument");
			}

			ctx.bus.emit(MenuCmd::Play(world_name));
		}

		if let Some(world_name) = ctx.console.command("save") {
			if world_name.is_empty() {
				anyhow::bail!("'save' requires world name argument");
			}

			ctx.vfs.save_json_resource(format!("worlds/{world_name}.world"), &self.model.world)
				.with_context(|| format!("Failed to save world '{world_name}'"))?;

			log::info!("World '{world_name}' saved successfully");
		}

		model::handle_hud_commands(ctx, &self.model)?;

		Ok(())
	}
}