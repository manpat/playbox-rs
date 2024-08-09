use crate::prelude::*;

pub struct GameScene {
	fog_shader: gfx::ShaderHandle,

	test_rt: gfx::ImageHandle,
	depth_rt: gfx::ImageHandle,

	toy_renderer: ToyRenderer,
	sprites: Sprites,
	world: world::World,
	world_view: world::WorldView,
	audio: MyAudioSystem,

	show_debug: bool,

	yaw: f32,
	pitch: f32,

	world_pos: world::WorldPosition,
	free_pos: Vec3,

	free_cam: bool,

	time: f32,
}

impl GameScene {
	pub fn new(ctx: &mut Context<'_>, audio: MyAudioSystem) -> anyhow::Result<GameScene> {
		let gfx::System{ core, resource_manager, .. } = &mut ctx.gfx;

		let test_rt = resource_manager.request(gfx::CreateImageRequest::rendertarget("test rendertarget", gfx::ImageFormat::hdr_color()));
		let depth_rt = resource_manager.request(gfx::CreateImageRequest::rendertarget("test depthbuffer", gfx::ImageFormat::Depth));

		let toy_renderer = {
			let project_path = resource_manager.resource_path().join("toys/basic.toy");
			let project_data = std::fs::read(&project_path)?;
			let project = toy::load(&project_data)?;

			let mut toy_renderer = ToyRenderer::new(&core, resource_manager);
			toy_renderer.set_color_target(test_rt);
			toy_renderer.set_depth_target(depth_rt);
			toy_renderer.update(&core, |builder| {
				builder.set_root_transform(Mat3x4::scale_translate(Vec3::splat(0.2), Vec3::from_y(0.3)));
				builder.add_entities(project.find_scene("main").unwrap());
			});
			toy_renderer
		};

		let world = world::World::new();

		Ok(GameScene {
			fog_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/fog.cs.glsl")?),

			test_rt,
			depth_rt,

			toy_renderer,
			sprites: Sprites::new(&mut ctx.gfx)?,

			audio,
			world_view: world::WorldView::new(&mut ctx.gfx, &world)?,
			world,

			show_debug: false,

			time: 0.0,
			yaw: 0.0,
			pitch: 0.0,

			world_pos: world::WorldPosition::default(),
			free_pos: Vec3::zero(),

			free_cam: false,
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		self.time += 1.0/60.0;

		if ctx.input.button_just_down(input::Key::F2) {
			self.show_debug = !self.show_debug;
		}

		ctx.input.set_capture_mouse(!self.show_debug);

		if self.show_debug {
			world::editor::draw_world_editor(&ctx.egui, &mut self.world, &mut self.world_view);
			return;
		}

		if ctx.input.button_just_down(input::Key::V) {
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

		let speed = match (ctx.input.button_down(input::Key::LShift), ctx.input.button_down(input::Key::LAlt)) {
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

			if ctx.input.button_down(input::Key::W) {
				self.free_pos += forward * speed;
			}

			if ctx.input.button_down(input::Key::S) {
				self.free_pos -= forward * speed;
			}

			if ctx.input.button_down(input::Key::D) {
				self.free_pos += right * speed;
			}

			if ctx.input.button_down(input::Key::A) {
				self.free_pos -= right * speed;
			}

		} else {
			let right = Vec2::from_angle(self.yaw);
			let forward = -right.perp();

			let mut delta = Vec2::zero();

			if ctx.input.button_down(input::Key::W) {
				delta += forward * speed;
			}

			if ctx.input.button_down(input::Key::S) {
				delta -= forward * speed;
			}

			if ctx.input.button_down(input::Key::D) {
				delta += right * speed;
			}

			if ctx.input.button_down(input::Key::A) {
				delta -= right * speed;
			}

			self.world.try_move_by(&mut self.world_pos, Some(&mut self.yaw), delta);
		}


		self.sprites.set_billboard_orientation(Vec3::from_y(1.0), Vec3::from_y_angle(self.yaw));
		// self.update_interactive_objects(ctx);
	}

	pub fn draw(&mut self, gfx: &mut gfx::System) {
		let aspect = gfx.backbuffer_aspect();
		let projection_view = Mat4::perspective(80.0f32.to_radians(), aspect, 0.01, 100.0)
			* Mat4::rotate_x(self.pitch)
			* Mat4::rotate_y(self.yaw)
			* Mat4::translate(-self.free_pos-Vec3::from_y(0.5));

		gfx.frame_encoder.backbuffer_color(self.world.fog_color);
		gfx.frame_encoder.bind_global_ubo(0, &[projection_view]);

		let mut main_group = gfx.frame_encoder.command_group(gfx::FrameStage::Main);
		main_group.bind_rendertargets(&[self.test_rt, self.depth_rt]);
		main_group.bind_shared_sampled_image(0, gfx.resource_manager.blank_white_image, gfx.resource_manager.nearest_sampler);

		self.world_view.draw(gfx, &mut self.sprites, &self.world, self.world_pos);

		// self.toy_renderer.draw(gfx);
		self.sprites.draw(gfx);

		self.dispatch_postprocess(gfx);
	}

	fn dispatch_postprocess(&self, gfx: &mut gfx::System) {
		let gfx::System { resource_manager: rm, frame_encoder, .. } = gfx;

		let mut group = frame_encoder.command_group(gfx::FrameStage::Postprocess);

		group.compute(self.fog_shader)
			.image_rw(0, self.test_rt)
			.sampled_image(1, self.depth_rt, rm.nearest_sampler)
			.groups_from_image_size(self.test_rt);

		group.draw_fullscreen(None)
			.sampled_image(0, self.test_rt, rm.nearest_sampler)
			.blend_mode(gfx::BlendMode::PREMULTIPLIED_ALPHA);
	}
}