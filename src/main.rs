#![feature(let_chains)]

use toybox::*;

mod world;

fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", App::new)
}



struct App {
	v_shader: gfx::ShaderHandle,
	v_basic_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,
	image_shader: gfx::ShaderHandle,

	toy_vertex_buffer: gfx::BufferName,
	toy_index_buffer: gfx::BufferName,
	toy_element_count: u32,

	image: gfx::ImageName,
	blank_image: gfx::ImageName,
	cool_image: gfx::ImageHandle,
	sampler: gfx::SamplerName,

	fbo: gfx::FramebufferName,
	test_rt: gfx::ImageHandle,

	sprites: Sprites,

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

		ctx.gfx.frame_encoder.backbuffer_color([1.0, 0.5, 1.0]);

		dbg!(&ctx.gfx.core.capabilities());
		dbg!(ctx.resource_root_path());

		// ctx.audio.set_provider(MyAudioProvider::default())?;

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
							BasicVertex { pos, color: Color::white(), .. Default::default() }
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
						BasicVertex { pos, color: Color::from(color), .. Default::default() }
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
			v_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/test.vs.glsl")?),
			v_basic_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/basic.vs.glsl")?),
			f_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/test.fs.glsl")?),
			image_shader: resource_manager.request(gfx::LoadShaderRequest::from("shaders/image.cs.glsl")?),

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

			blank_image: {
				let format = gfx::ImageFormat::Rgba(gfx::ComponentFormat::Unorm8);
				let image = core.create_image_2d(format, Vec2i::splat(1));
				core.upload_image(image, None, format, &[255u8, 255, 255, 255]);
				core.set_debug_label(image, "Blank white image");
				image
			},

			cool_image: resource_manager.request(gfx::LoadImageRequest::from("images/coolcat.png")),

			fbo: {
				let name = core.create_framebuffer();
				core.set_debug_label(name, "test fbo");
				name
			},

			test_rt: resource_manager.request(gfx::CreateImageRequest::rendertarget("test rendertarget", gfx::ImageFormat::Srgba8)),

			sampler: {
				use gfx::{FilterMode, AddressingMode};

				let sampler = core.create_sampler();
				core.set_sampler_minify_filter(sampler, FilterMode::Nearest, None);
				core.set_sampler_magnify_filter(sampler, FilterMode::Nearest);
				core.set_sampler_addressing_mode(sampler, AddressingMode::Clamp);
				core.set_sampler_axis_addressing_mode(sampler, gfx::Axis::X, AddressingMode::Repeat);
				core.set_debug_label(sampler, "Test sampler");
				sampler
			},

			sprites: Sprites::new(&mut ctx.gfx)?,

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


		let aspect = ctx.gfx.backbuffer_aspect();
		let projection = Mat4::perspective(80.0f32.to_radians(), aspect, 0.01, 100.0)
			* Mat4::rotate_x(self.pitch)
			* Mat4::rotate_y(self.yaw)
			* Mat4::translate(-Vec3::from_y(0.5) - self.pos.to_x0y());

		ctx.gfx.frame_encoder.bind_global_ubo(1, &[projection]);

		egui::Window::new("Wahoo")
			.resizable(false)
			.show(&ctx.egui, |ui| {
				ui.label("Hello egui!");
				ui.label("Text text text!");
				if ui.button("Huh??").clicked() {
					ctx.show_debug_menu = true;
				}

				ui.checkbox(&mut ctx.show_debug_menu, "Show debug menu");

				egui_backend::show_image_handle(ui, self.test_rt);

				let (response, painter) = ui.allocate_painter(egui::Vec2::splat(100.0), egui::Sense::hover());

				let rect = response.rect;
				let c = rect.center();
				let mut r = rect.width() / 2.0 - 1.0;
				let color = egui::Color32::from_gray(128);
				let stroke = egui::Stroke::new(1.0, color);

				painter.with_clip_rect(egui::Rect::from_min_size(c, rect.size() / 2.0))
					.circle_filled(c, r, egui::Color32::RED);

				for _ in 0..20 {
					painter.circle_stroke(c, r, stroke);
					r *= 0.9;
				}
			});

		ctx.gfx.frame_encoder.command_group("Rotate image colours")
			.compute(self.image_shader)
			.groups([3, 3, 1])
			.image_rw(0, self.image);

		let mut group = ctx.gfx.frame_encoder.command_group("Draw everything");
		group.bind_shared_sampled_image(0, self.image, self.sampler);

		self.time += 1.0/60.0;
		
		let fbo = self.fbo;
		let test_rt = self.test_rt;
		group.execute(move |core, rm| {
			let test_rt_name = rm.images.get_name(test_rt).unwrap();

			core.set_framebuffer_attachment(fbo, gfx::FramebufferAttachment::Color(0), test_rt_name);

			core.clear_framebuffer_color_buffer(fbo, 0, [0.0; 4]);
			core.bind_framebuffer(fbo);
		});

		group.draw(self.v_basic_shader, self.f_shader)
			.indexed(self.toy_index_buffer)
			.ssbo(0, self.toy_vertex_buffer)
			.sampled_image(0, self.blank_image, self.sampler)
			.elements(self.toy_element_count);

		group.execute(move |core, _| {
			core.bind_framebuffer(None);
		});

		if let Some(pos) = ctx.input.pointer_position()
			&& !ctx.input.button_down(input::MouseButton::Left)
		{
			let pos = (pos * Vec2::new(aspect, 1.0)).extend(-0.5);

			let projection = Mat4::ortho_aspect(1.0, aspect, 0.01, 100.0);
			let rot = Mat3x4::rotate_z(self.time);

			let vertices = [
				BasicVertex {
					pos: pos + rot * Vec3::new(0.0, 0.1, 0.0),
					uv: Vec2::new(1.0, 1.0),
					color: Color::white(), 
					.. BasicVertex::default()
				},
				BasicVertex {
					pos: pos + rot * Vec3::new(0.0,-0.1, 0.0),
					uv: Vec2::new(0.0, 0.0),
					color: Color::white(),
					.. BasicVertex::default()
				},
				BasicVertex {
					pos: pos + rot * Vec3::new(0.1, 0.0, 0.0),
					uv: Vec2::new(1.0, 0.0),
					color: Color::white(),
					.. BasicVertex::default()
				},
				BasicVertex {
					pos: pos + rot * Vec3::new(-0.1, 0.0, 0.0),
					uv: Vec2::new(0.0, 1.0),
					color: Color::white(),
					.. BasicVertex::default()
				},
			];

			group.draw(self.v_basic_shader, self.f_shader)
				.elements(6)
				.ubo(1, &[projection])
				.ssbo(0, &vertices)
				.indexed(&[0u32, 2, 3, 2, 1, 3]);
		}


		let up = Vec3::from_y(1.0);
		let right = Vec3::from_y_angle(self.yaw);

		// Ground
		self.sprites.basic(Vec3::from_x(10.0), Vec3::from_z(-10.0), Vec3::from_z(5.0), Color::rgb(0.5, 0.5, 0.5));

		for (key, &world::Object{pos, size, color, ..}) in self.world.objects.iter() {
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
	}

	fn customise_debug_menu(&mut self, ui: &mut egui::Ui) {
		ui.menu_button("Playbox", |ui| {
			let _ = ui.button("???");
		});
	}
}


#[derive(Default)]
struct MyAudioProvider {
	sample_dt: f64,
	phase: f64,
}

impl audio::Provider for MyAudioProvider {
	fn on_configuration_changed(&mut self, config: audio::Configuration) {
		self.sample_dt = 1.0/config.sample_rate as f64;
		assert!(config.channels == 2);
	}

	fn fill_buffer(&mut self, buffer: &mut [f32]) {
		let mut osc_phase = self.phase * 220.0 * std::f64::consts::TAU;
		let osc_dt = self.sample_dt * 220.0 * std::f64::consts::TAU;

		for frame in buffer.chunks_exact_mut(2) {
			let osc = osc_phase.sin();
			let amp = 0.5 - self.phase.cos() * 0.5;
			let amp = amp * amp;

			let value = (amp * osc) as f32;

			frame[0] = value;
			frame[1] = -value;

			self.phase += self.sample_dt;
			osc_phase += osc_dt;
		}

		self.phase %= std::f64::consts::TAU;
	}
}


#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct BasicVertex {
	pos: Vec3, _pad: f32,
	color: Color,
	uv: Vec2, _pad2: [f32; 2],
}

#[derive(Debug)]
struct Sprites {
	vertices: Vec<BasicVertex>,
	indices: Vec<u32>,

	v_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,

	atlas: gfx::ImageHandle,
	sampler: gfx::SamplerName,
}

impl Sprites {
	fn new(gfx: &mut gfx::System) -> anyhow::Result<Sprites> {
		Ok(Sprites {
			vertices: Vec::new(),
			indices: Vec::new(),

			v_shader: gfx.resource_manager.request(gfx::LoadShaderRequest::from("shaders/basic.vs.glsl")?),
			f_shader: gfx.resource_manager.request(gfx::LoadShaderRequest::from("shaders/test.fs.glsl")?),

			atlas: gfx.resource_manager.request(gfx::LoadImageRequest::from("images/coolcat.png")),

			sampler: {
				let sampler = gfx.core.create_sampler();
				gfx.core.set_sampler_minify_filter(sampler, gfx::FilterMode::Nearest, None);
				gfx.core.set_sampler_magnify_filter(sampler, gfx::FilterMode::Nearest);
				gfx.core.set_sampler_addressing_mode(sampler, gfx::AddressingMode::Clamp);
				gfx.core.set_debug_label(sampler, "Sprite sampler");
				sampler
			},
		})
	}

	fn draw(&mut self, gfx: &mut gfx::System) {
		if self.vertices.is_empty() {
			return
		}

		gfx.frame_encoder.command_group("Sprites")
			.draw(self.v_shader, self.f_shader)
			.elements(self.indices.len() as u32)
			.indexed(&self.indices)
			.ssbo(0, &self.vertices)
			.sampled_image(0, self.atlas, self.sampler);

		self.vertices.clear();
		self.indices.clear();
	}
}

impl Sprites {
	fn basic(&mut self, right: Vec3, up: Vec3, pos: Vec3, color: Color) {
		let start_index = self.vertices.len() as u32;
		let indices = [0, 1, 2, 0, 2, 3].into_iter().map(|i| i + start_index);

		let right = right/2.0;

		let vertices = [
			BasicVertex {
				pos: pos - right,
				uv: Vec2::new(0.0, 0.0),
				color,
				.. BasicVertex::default()
			},
			BasicVertex {
				pos: pos - right + up,
				uv: Vec2::new(0.0, 1.0),
				color,
				.. BasicVertex::default()
			},
			BasicVertex {
				pos: pos + right + up,
				uv: Vec2::new(1.0, 1.0),
				color, 
				.. BasicVertex::default()
			},
			BasicVertex {
				pos: pos + right,
				uv: Vec2::new(1.0, 0.0),
				color,
				.. BasicVertex::default()
			},
		];

		self.vertices.extend_from_slice(&vertices);
		self.indices.extend(indices);
	}
}