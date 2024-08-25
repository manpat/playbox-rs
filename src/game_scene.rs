use crate::prelude::*;

pub struct GameScene {
	fog_shader: gfx::ShaderHandle,
	hdr_to_ldr_shader: gfx::ShaderHandle,

	hdr_color_rt: gfx::ImageHandle,
	depth_rt: gfx::ImageHandle,

	ldr_color_image: gfx::ImageHandle,

	// toy_renderer: ToyRenderer,
	sprites: Sprites,
	world: world::World,
	world_view: world_view::WorldView,

	message_bus: MessageBus,
	show_debug: bool,

	yaw: f32,
	pitch: f32,

	world_pos: world::WorldPosition,
	free_pos: Vec3,

	free_cam: bool,

	time: f32,

	editor_state: editor::State,
}

impl GameScene {
	pub fn new(ctx: &mut Context<'_>, world: world::World) -> anyhow::Result<GameScene> {
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

		Ok(GameScene {
			fog_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/fog.cs.glsl")?),
			hdr_to_ldr_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/hdr_to_ldr.cs.glsl")?),

			hdr_color_rt,
			depth_rt,

			ldr_color_image,

			// toy_renderer,
			sprites: Sprites::new(&mut ctx.gfx)?,

			world_view: world_view::WorldView::new(&mut ctx.gfx, &world, ctx.message_bus.clone())?,
			world,

			message_bus: ctx.message_bus.clone(),

			show_debug: false,

			yaw: 0.0,
			pitch: 0.0,

			world_pos: world::WorldPosition::default(),
			free_pos: Vec3::zero(),

			free_cam: false,

			time: 0.0,

			editor_state: editor::State::new(ctx.message_bus),
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		if ctx.input.button_just_down(input::keys::F2) {
			self.show_debug = !self.show_debug;
		}

		ctx.input.set_capture_mouse(!self.show_debug);

		if self.show_debug {
			editor::draw_world_editor(&ctx.egui, &mut self.editor_state, &self.world, &self.message_bus);
			editor::handle_editor_cmds(&self.editor_state, &mut self.world, &self.message_bus);
			return;
		}

		if ctx.input.button_just_down(input::keys::KeyV) {
			self.free_cam = !self.free_cam;

			if !self.free_cam {
				self.free_pos = Vec3::zero();
			}
		}

		// TODO(pat.m): factor out camera/player controller stuff
		{
			let (dx, dy) = ctx.input.mouse_delta().map_or((0.0, 0.0), Vec2::to_tuple);
			self.yaw += dx * TAU;
			self.yaw %= TAU;

			let pitch_limit = PI/2.0;
			self.pitch = (self.pitch - 3.0*dy).clamp(-pitch_limit, pitch_limit);
		}

		let speed = match (ctx.input.button_down(input::keys::Shift), ctx.input.button_down(input::keys::Alt)) {
			(true, false) => 4.0 / 60.0,
			(false, true) => 0.5 / 60.0,
			_ => 2.0 / 60.0,
		};

		if self.free_cam {
			// TODO(pat.m): figure out why these need to be negated :(
			// yaw at least I think is because I'm using Vec2::to_x0y, but pitch??
			let yaw_orientation = Quat::from_yaw(-self.yaw);
			let orientation = yaw_orientation * Quat::from_pitch(-self.pitch);

			let right = yaw_orientation.right();
			let forward = orientation.forward();

			if ctx.input.button_down(input::keys::KeyW) {
				self.free_pos += forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyS) {
				self.free_pos -= forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyD) {
				self.free_pos += right * speed;
			}

			if ctx.input.button_down(input::keys::KeyA) {
				self.free_pos -= right * speed;
			}

		} else {
			let right = Vec2::from_angle(self.yaw);
			let forward = -right.perp();

			let mut delta = Vec2::zero();

			if ctx.input.button_down(input::keys::KeyW) {
				delta += forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyS) {
				delta -= forward * speed;
			}

			if ctx.input.button_down(input::keys::KeyD) {
				delta += right * speed;
			}

			if ctx.input.button_down(input::keys::KeyA) {
				delta -= right * speed;
			}

			self.world.try_move_by(&mut self.world_pos, Some(&mut self.yaw), delta);
		}


		self.sprites.set_billboard_orientation(Vec3::from_y(1.0), Vec3::from_y_angle(self.yaw));
		// self.update_interactive_objects(ctx);
	}

	pub fn draw(&mut self, gfx: &mut gfx::System) {
		self.time += 1.0/60.0;

		let aspect = gfx.backbuffer_aspect();
		let projection = Mat4::perspective(80.0f32.to_radians(), aspect, 0.01, 100.0);
		let projection_view = projection
			* Mat4::rotate_x(self.pitch)
			* Mat4::rotate_y(self.yaw)
			* Mat4::translate(-self.free_pos-Vec3::from_y(0.5));

		let inverse_projection = projection.inverse();

		gfx.frame_encoder.backbuffer_color(self.world.fog_color);
		gfx.frame_encoder.bind_global_ubo(0, &[projection_view, inverse_projection]);

		let mut main_group = gfx.frame_encoder.command_group(gfx::FrameStage::Main);
		main_group.bind_rendertargets(&[self.hdr_color_rt, self.depth_rt]);
		main_group.bind_shared_sampled_image(0, gfx.resource_manager.blank_white_image, gfx.resource_manager.nearest_sampler);

		self.world_view.draw(gfx, &mut self.sprites, &self.world, self.world_pos);

		// self.toy_renderer.draw(gfx);
		self.sprites.draw(gfx);

		self.dispatch_postprocess(gfx);
	}

	fn dispatch_postprocess(&self, gfx: &mut gfx::System) {
		let gfx::System { resource_manager: rm, frame_encoder, .. } = gfx;

		let mut group = frame_encoder.command_group(gfx::FrameStage::Postprocess);

		#[repr(C)]
		#[derive(Copy, Clone)]
		struct FogParameters {
			fog_color: Color,
		}

		group.compute(self.fog_shader)
			.image_rw(0, self.hdr_color_rt)
			.sampled_image(1, self.depth_rt, rm.nearest_sampler)
			.ubo(1, &[FogParameters {
				fog_color: self.world.fog_color
			}])
			.groups_from_image_size(self.hdr_color_rt);

		// TODO(pat.m): bloom
		// TODO(pat.m): tone map

		group.compute(self.hdr_to_ldr_shader)
			.image(0, self.hdr_color_rt)
			.image_rw(1, self.ldr_color_image)
			.ssbo(0, &[self.time])
			.groups_from_image_size(self.hdr_color_rt);

		// TODO(pat.m): blit
		group.draw_fullscreen(None)
			.sampled_image(0, self.ldr_color_image, rm.nearest_sampler);
	}
}




impl GameScene {
	pub fn add_editor_debug_menu(&mut self, ui: &mut egui::Ui) {
		let default_world_path = "resource/worlds/default.world";

		ui.menu_button("Editor", |ui| {
			if ui.button("New World").clicked() {
				self.world = world::World::new();
				self.message_bus.emit(world::WorldChangedEvent);

				// TODO(pat.m): switch to Game state

				ui.close_menu();
			}

			if ui.button("New Default World").clicked() {
				self.world = world::World::new_old();
				self.message_bus.emit(world::WorldChangedEvent);
				// TODO(pat.m): switch to Game state

				ui.close_menu();
			}

			if ui.button("Load World").clicked() {
				// TODO(pat.m): get resource manager to find load/save path

				match world::World::load(default_world_path) {
					Ok(new_world) => {
						self.world = new_world;
						self.message_bus.emit(world::WorldChangedEvent);
						// TODO(pat.m): switch to Game state
					}

					Err(error) => {
						eprintln!("Failed to load world '{default_world_path}': {error}");
					}
				}

				ui.close_menu();
			}

			if ui.button("Save World").clicked() {
				if let Err(error) = self.world.save(default_world_path) {
					eprintln!("Failed to save world to '{default_world_path}': {error}");
				}

				ui.close_menu();
			}
		});
	}
}