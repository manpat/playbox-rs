use crate::prelude::*;

pub struct GameScene {
	posteffect_shader: gfx::ShaderHandle,

	test_rt: gfx::ImageHandle,
	test2_rt: gfx::ImageHandle,
	depth_rt: gfx::ImageHandle,

	toy_renderer: ToyRenderer,
	sprites: Sprites,
	world: world::World,
	audio: MyAudioSystem,

	yaw: f32,
	pitch: f32,
	pos: Vec2,

	time: f32,
}

impl GameScene {
	pub fn new(ctx: &mut toybox::Context, audio: MyAudioSystem) -> anyhow::Result<GameScene> {
		let gfx::System{ core, resource_manager, .. } = &mut ctx.gfx;

		let test_rt = resource_manager.request(gfx::CreateImageRequest::rendertarget("test rendertarget", gfx::ImageFormat::Rgb10A2));
		let test2_rt = resource_manager.request(gfx::CreateImageRequest::rendertarget("test rendertarget 2", gfx::ImageFormat::Rgb10A2));
		let depth_rt = resource_manager.request(gfx::CreateImageRequest::rendertarget("test depthbuffer", gfx::ImageFormat::Depth));

		let toy_renderer = {
			let project_path = resource_manager.resource_path().join("toys/basic.toy");
			let project_data = std::fs::read(&project_path)?;
			let project = toy::load(&project_data)?;

			let mut toy_renderer = ToyRenderer::new(&core, resource_manager);
			toy_renderer.set_color_target(test2_rt);
			toy_renderer.set_depth_target(depth_rt);
			toy_renderer.update(&core, |builder| {
				builder.set_root_transform(Mat3x4::scale_translate(Vec3::splat(0.2), Vec3::from_y(0.3)));
				builder.add_entities(project.find_scene("main").unwrap());
			});
			toy_renderer
		};

		Ok(GameScene {
			posteffect_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/post.cs.glsl")?),

			test_rt,
			test2_rt,
			depth_rt,

			toy_renderer,
			sprites: Sprites::new(&mut ctx.gfx)?,

			audio,
			world: world::make_test_world(),

			time: 0.0,
			yaw: 0.0,
			pitch: 0.0,

			pos: Vec2::from_y(3.0),
		})
	}

	pub fn update(&mut self, ctx: &mut toybox::Context) {
		self.time += 1.0/60.0;

		self.world.update();

		ctx.input.set_capture_mouse(true);

		// TODO(pat.m): factor out camera/player controller stuff
		// TODO(pat.m): Allow free cam
		/*if ctx.input.button_down(input::MouseButton::Left)*/ {
			let (dx, dy) = ctx.input.mouse_delta().map_or((0.0, 0.0), Vec2::to_tuple);
			self.yaw += dx * TAU;
			self.yaw %= TAU;

			let pitch_limit = PI/2.0;
			self.pitch = (self.pitch - 3.0*dy).clamp(-pitch_limit, pitch_limit);
		}

		let right = Vec2::from_angle(self.yaw);
		let forward = -right.perp();
		let speed = match ctx.input.button_down(input::Key::LShift) {
			true => 4.0 / 60.0,
			false => 2.0 / 60.0,
		};

		if ctx.input.button_down(input::Key::W) {
			self.pos += forward * speed;
		}

		if ctx.input.button_down(input::Key::S) {
			self.pos -= forward * speed;
		}

		if ctx.input.button_down(input::Key::D) {
			self.pos += right * speed;
		}

		if ctx.input.button_down(input::Key::A) {
			self.pos -= right * speed;
		}

		if ctx.input.button_just_down(input::Key::F) {
			self.audio.trigger();
		}

		self.sprites.set_billboard_orientation(Vec3::from_y(1.0), Vec3::from_y_angle(self.yaw));
		self.update_interactive_objects(ctx);
	}

	pub fn draw(&mut self, gfx: &mut gfx::System) {
		let aspect = gfx.backbuffer_aspect();
		let projection = Mat4::perspective(80.0f32.to_radians(), aspect, 0.01, 100.0)
			* Mat4::rotate_x(self.pitch)
			* Mat4::rotate_y(self.yaw)
			* Mat4::translate(-Vec3::from_y(0.5) - self.pos.to_x0y());

		gfx.frame_encoder.backbuffer_color(Color::light_magenta());
		gfx.frame_encoder.bind_global_ubo(0, &[projection]);

		gfx.frame_encoder.command_group(gfx::FrameStage::Main)
			.bind_rendertargets(&[self.test_rt, self.depth_rt]);

		self.draw_world();

		self.toy_renderer.draw(gfx);
		self.sprites.draw(gfx);

		self.dispatch_postprocess(gfx);
	}

	fn draw_world(&mut self) {
		// Ground
		self.sprites.basic(Vec3::from_x(10.0), Vec3::from_z(-10.0), Vec3::from_z(5.0), Color::rgb(0.5, 0.5, 0.5));

		// Objects
		for (_, &world::Object{pos, size, color, ..}) in self.world.objects.iter() {
			self.sprites.billboard(pos, size, color);
		}
	}

	fn update_interactive_objects(&mut self, ctx: &mut toybox::Context) {
		let eye = Vec3::from_y(0.5) + self.pos.to_x0y();
		let dir = Vec3::from_y_angle(self.yaw - PI/2.0);

		if let Some(key) = self.world.nearest_interactive(eye, dir) {
			let &world::Object{pos, size, ..} = &self.world.objects[key];
			self.sprites.billboard(pos + Vec3::from_y(size.y + 0.05), Vec2::splat(0.1), Color::white());

			if ctx.input.button_just_down(input::Key::Space) || ctx.input.button_just_down(input::MouseButton::Right) {
				self.world.interact(key);
			}
		}
	}

	fn dispatch_postprocess(&self, gfx: &mut gfx::System) {
		let gfx::System { resource_manager: rm, frame_encoder, .. } = gfx;

		let mut group = frame_encoder.command_group(gfx::FrameStage::Postprocess);

		group.compute(self.posteffect_shader)
			.image_rw(0, self.test2_rt)
			.groups_from_image_size(self.test2_rt);

		group.draw_fullscreen(None)
			.sampled_image(0, self.test2_rt, rm.nearest_sampler)
			.blend_mode(gfx::BlendMode::ALPHA);

		group.draw_fullscreen(None)
			.sampled_image(0, self.test_rt, rm.nearest_sampler)
			.blend_mode(gfx::BlendMode::ALPHA);
	}
}