#![feature(let_chains)]

use toybox::*;

mod audio;
mod sprites;
mod world;

use audio::MyAudioSystem;
use sprites::Sprites;


fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_BACKTRACE", "1");
	toybox::run("playbox", App::new)
}



struct App {
	posteffect_shader: gfx::ShaderHandle,

	toy_vertex_buffer: gfx::BufferName,
	toy_index_buffer: gfx::BufferName,
	toy_element_count: u32,

	image: gfx::ImageName,

	test_rt: gfx::ImageHandle,
	test2_rt: gfx::ImageHandle,
	depth_rt: gfx::ImageHandle,

	sprites: Sprites,

	audio: MyAudioSystem,
	world: world::World,

	time: f32,
	yaw: f32,
	pitch: f32,
	pos: Vec2,

	// show_sprites: bool,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.show_debug_menu = true;

		ctx.gfx.frame_encoder.backbuffer_color(Color::light_magenta());

		dbg!(&ctx.gfx.core.capabilities());
		dbg!(ctx.resource_root_path());

		let audio = MyAudioSystem::start(&mut ctx.audio)?;

		let gfx::System{ core, resource_manager, .. } = &mut ctx.gfx;

		let toy_vertex_buffer;
		let toy_index_buffer;
		let toy_element_count;

		{
			let project_path = resource_manager.resource_path().join("toys/basic.toy");
			let project_data = std::fs::read(&project_path)?;
			let project = toy::load(&project_data)?;

			let mut vertex_data = Vec::new();
			let mut index_data = Vec::new();

			for entity in project.entities() {
				let Some(mesh) = entity.mesh() else {
					println!("Entity {} had no mesh - skipping", entity.name);
					continue
				};

				let transform = Mat3x4::scale_translate(Vec3::splat(0.2), Vec3::from_y(0.3)) * entity.transform();

				if mesh.color_layers.is_empty() {
					println!("Entity {} had no color data - setting to white", entity.name);

					let vertices = mesh.positions.iter()
						.map(|&pos| {
							let pos = transform * pos;
							gfx::StandardVertex::from_pos(pos)
						});

					let index_start = vertex_data.len() as u32;
					let indices = mesh.indices.iter().map(|idx| *idx as u32 + index_start);

					vertex_data.extend(vertices);
					index_data.extend(indices);

					continue;
				}

				let vertices = std::iter::zip(&mesh.positions, &mesh.color_layers[0].data)
					.map(|(&pos, &color)| {
						let pos = transform * pos;
						gfx::StandardVertex::with_color(pos, color)
					});

				let index_start = vertex_data.len() as u32;
				let indices = mesh.indices.iter().map(|idx| *idx as u32 + index_start);

				vertex_data.extend(vertices);
				index_data.extend(indices);
			}

			toy_vertex_buffer = core.create_buffer();
			toy_index_buffer = core.create_buffer();
			toy_element_count = index_data.len() as u32;

			core.upload_immutable_buffer_immediate(toy_vertex_buffer, &vertex_data);
			core.upload_immutable_buffer_immediate(toy_index_buffer, &index_data);
		}

		Ok(App {
			posteffect_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/post.cs.glsl")?),

			toy_vertex_buffer,
			toy_index_buffer,
			toy_element_count,

			image: {
				let format = gfx::ImageFormat::Rgba(gfx::ComponentFormat::Unorm8);
				let image = core.create_image_2d(format, Vec2i::new(3, 3));
				core.upload_image(image, None, format, &[
					 20u8, 255, 255, 255,
					255,  20, 255, 255,
					255, 255,  20, 255,

					255,  20,  20, 255,
					 20, 255,  20, 255,
					 20,  20, 255, 255,

					255, 255, 255, 255,
					100, 100, 100, 255,
					 20,  20,  20, 255,
				]);
				core.set_debug_label(image, "Test image");

				image
			},

			test_rt: resource_manager.request(gfx::CreateImageRequest::rendertarget("test rendertarget", gfx::ImageFormat::Rgb10A2)),
			test2_rt: resource_manager.request(gfx::CreateImageRequest::rendertarget("test rendertarget 2", gfx::ImageFormat::Rgb10A2)),
			depth_rt: resource_manager.request(gfx::CreateImageRequest::rendertarget("test depthbuffer", gfx::ImageFormat::Depth)),

			sprites: Sprites::new(&mut ctx.gfx)?,

			audio,
			world: world::make_test_world(),

			time: 0.0,
			yaw: 0.0,
			pitch: 0.0,

			pos: Vec2::from_y(3.0),
		})
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		self.world.update();

		if ctx.input.button_just_down(input::MouseButton::Left) {
			ctx.input.set_capture_mouse(true);
		}

		if ctx.input.button_just_up(input::MouseButton::Left) {
			ctx.input.set_capture_mouse(false);
		}

		if ctx.input.button_down(input::MouseButton::Left) {
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


		let aspect = ctx.gfx.backbuffer_aspect();
		let projection = Mat4::perspective(80.0f32.to_radians(), aspect, 0.01, 100.0)
			* Mat4::rotate_x(self.pitch)
			* Mat4::rotate_y(self.yaw)
			* Mat4::translate(-Vec3::from_y(0.5) - self.pos.to_x0y());

		ctx.gfx.frame_encoder.bind_global_ubo(0, &[projection]);

		self.time += 1.0/60.0;

		let rm = &mut ctx.gfx.resource_manager;

		let mut group = ctx.gfx.frame_encoder.command_group(gfx::FrameStage::Main);
		group.bind_shared_sampled_image(0, self.image, rm.nearest_sampler);
		group.bind_rendertargets(&[self.test_rt, self.depth_rt]);

		group.clear_image_to_default(self.test_rt);
		group.clear_image_to_default(self.test2_rt);
		group.clear_image_to_default(self.depth_rt);

		group.draw(rm.standard_vs_shader, rm.flat_fs_shader)
			.indexed(self.toy_index_buffer)
			.ssbo(0, self.toy_vertex_buffer)
			.sampled_image(0, rm.blank_white_image, rm.nearest_sampler)
			.elements(self.toy_element_count)
			.rendertargets(&[self.test2_rt, self.depth_rt]);

		if let Some(pos) = ctx.input.mouse_position_ndc()
			&& !ctx.input.button_down(input::MouseButton::Left)
		{
			let pos = (pos * Vec2::new(aspect, 1.0)).extend(-0.5);

			let projection = Mat4::ortho_aspect(1.0, aspect, 0.01, 100.0);
			let rot = Mat3x4::rotate_z(self.time);

			let vertices = [
				gfx::StandardVertex::with_uv(pos + rot * Vec3::new(0.0, 0.1, 0.0), Vec2::new(1.0, 1.0)),
				gfx::StandardVertex::with_uv(pos + rot * Vec3::new(0.0,-0.1, 0.0), Vec2::new(0.0, 0.0)),
				gfx::StandardVertex::with_uv(pos + rot * Vec3::new(0.1, 0.0, 0.0), Vec2::new(1.0, 0.0)),
				gfx::StandardVertex::with_uv(pos + rot * Vec3::new(-0.1, 0.0, 0.0), Vec2::new(0.0, 1.0)),
			];

			group.draw(rm.standard_vs_shader, rm.flat_fs_shader)
				.elements(6)
				.ubo(0, &[projection])
				.ssbo(0, &vertices)
				.indexed(&[0u32, 2, 3, 2, 1, 3]);
		}


		let up = Vec3::from_y(1.0);
		let right = Vec3::from_y_angle(self.yaw);

		// Ground
		self.sprites.basic(Vec3::from_x(10.0), Vec3::from_z(-10.0), Vec3::from_z(5.0), Color::rgb(0.5, 0.5, 0.5));

		for (_, &world::Object{pos, size, color, ..}) in self.world.objects.iter() {
			self.sprites.basic(right * size.x, up * size.y, pos, color);
		}


		let eye = Vec3::from_y(0.5) + self.pos.to_x0y();
		let dir = Vec3::from_y_angle(self.yaw - PI/2.0);

		if let Some(key) = self.world.nearest_interactive(eye, dir) {
			let &world::Object{pos, size, ..} = &self.world.objects[key];
			self.sprites.basic(right * 0.1, up * 0.1, pos + up * (size.y + 0.05), Color::white());

			if ctx.input.button_just_down(input::Key::Space) || ctx.input.button_just_down(input::MouseButton::Right) {
				self.world.interact(key);
			}
		}

		self.sprites.draw(&mut ctx.gfx);

		let rm = &mut ctx.gfx.resource_manager;
		let mut postprocess_group = ctx.gfx.frame_encoder.command_group(gfx::FrameStage::Postprocess);

		postprocess_group.compute(self.posteffect_shader)
			.image_rw(0, self.test2_rt)
			.groups_from_image_size(self.test2_rt);

		postprocess_group.draw(rm.fullscreen_vs_shader, rm.flat_fs_shader)
			.sampled_image(0, self.test2_rt, rm.nearest_sampler)
			.elements(6)
			.blend_mode(gfx::BlendMode::ALPHA)
			.depth_test(false);

		postprocess_group.draw(rm.fullscreen_vs_shader, rm.flat_fs_shader)
			.sampled_image(0, self.test_rt, rm.nearest_sampler)
			.elements(6)
			.blend_mode(gfx::BlendMode::ALPHA)
			.depth_test(false);
	}

	fn customise_debug_menu(&mut self, ui: &mut egui::Ui) {
		ui.menu_button("Playbox", |ui| {
			let _ = ui.button("???");
		});
	}
}



#[derive(Debug)]
pub struct ToyRenderer {
	texture: gfx::ImageNameOrHandle,
	rendertarget: Option<gfx::ImageNameOrHandle>,
	framestage: gfx::FrameStage,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,
}

impl ToyRenderer {
	pub fn new(rm: &mut gfx::ResourceManager) -> ToyRenderer {
		ToyRenderer {
			texture: rm.blank_white_image.into(),
			rendertarget: None,
			framestage: gfx::FrameStage::Main,

			v_shader: rm.standard_vs_shader,
			f_shader: rm.flat_fs_shader,
		}
	}
}