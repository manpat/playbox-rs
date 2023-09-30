use toybox::*;

fn main() -> anyhow::Result<()> {
	// std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", App::new)
}



struct App {
	v_shader: gfx::ShaderHandle,
	v_basic_shader: gfx::ShaderHandle,
	f_shader: gfx::ShaderHandle,
	c_shader: gfx::ShaderHandle,
	image_shader: gfx::ShaderHandle,

	vertex_buffer: gfx::BufferName,
	line_index_buffer: gfx::BufferName,

	image: gfx::ImageName,
	cool_image: gfx::ImageHandle,
	sampler: gfx::SamplerName,

	sprites: Sprites,

	time: f32,
	yaw: f32,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.show_debug_menu = true;

		ctx.gfx.frame_encoder.backbuffer_color([1.0, 0.5, 1.0]);

		// ctx.audio.set_provider(MyAudioProvider::default())?;

		dbg!(&ctx.gfx.core.capabilities());

		let mut group = ctx.gfx.frame_encoder.command_group("START");
		group.debug_marker("FUCK");

		use gfx::resource_manager::LoadShaderRequest;

		Ok(App {
			v_shader: ctx.gfx.resource_manager.request(LoadShaderRequest::from("shaders/test.vs.glsl")?),
			v_basic_shader: ctx.gfx.resource_manager.request(LoadShaderRequest::from("shaders/basic.vs.glsl")?),
			f_shader: ctx.gfx.resource_manager.request(LoadShaderRequest::from("shaders/test.fs.glsl")?),
			c_shader: ctx.gfx.resource_manager.request(LoadShaderRequest::from("shaders/test.cs.glsl")?),
			image_shader: ctx.gfx.resource_manager.request(LoadShaderRequest::from("shaders/image.cs.glsl")?),

			// TODO(pat.m): these will go away with the temporary storage heap
			vertex_buffer: {
				let buffer = ctx.gfx.core.create_buffer();
				let flags = 0; // not client visible
				// TODO(pat.m): using Vec4 here to cope with std140 layout. this is an easy mistake to make
				// how to make this better?
				ctx.gfx.core.allocate_buffer_storage(buffer, std::mem::size_of::<[Vec4; 3]>(), flags);
				ctx.gfx.core.set_debug_label(buffer, "compute vertex buffer");
				buffer
			},

			line_index_buffer: {
				let buffer = ctx.gfx.core.create_buffer();
				let flags = 0; // not client visible
				ctx.gfx.core.allocate_buffer_storage(buffer, std::mem::size_of::<[u32; 6]>(), flags);
				ctx.gfx.core.set_debug_label(buffer, "compute index buffer");
				buffer
			},

			image: {
				let format = gfx::ImageFormat::Rgba(gfx::ComponentFormat::Unorm8);
				let image = ctx.gfx.core.create_image_2d(format, Vec2i::new(3, 3));
				ctx.gfx.core.upload_image(image, None, format, &[
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
				ctx.gfx.core.set_debug_label(image, "Test image");

				image
			},

			cool_image: ctx.gfx.resource_manager.request(gfx::LoadImageRequest::from("images/coolcat.png")),

			sampler: {
				use gfx::{FilterMode, AddressingMode};

				let sampler = ctx.gfx.core.create_sampler();
				ctx.gfx.core.set_sampler_minify_filter(sampler, FilterMode::Nearest, None);
				ctx.gfx.core.set_sampler_magnify_filter(sampler, FilterMode::Nearest);
				ctx.gfx.core.set_sampler_addressing_mode(sampler, AddressingMode::Clamp);
				ctx.gfx.core.set_sampler_axis_addressing_mode(sampler, gfx::Axis::X, AddressingMode::Repeat);
				ctx.gfx.core.set_debug_label(sampler, "Test sampler");
				sampler
			},

			sprites: Sprites::new(&mut ctx.gfx)?,

			time: 0.0,
			yaw: 0.0,
		})
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		if ctx.input.button_just_down(input::MouseButton::Left) {
			ctx.input.set_capture_mouse(true);
		}

		if ctx.input.button_just_up(input::MouseButton::Left) {
			ctx.input.set_capture_mouse(false);
		}

		if ctx.input.button_down(input::MouseButton::Left) {
			let dx = ctx.input.mouse_delta().map_or(0.0, |delta| delta.x);
			self.yaw += dx * TAU;
			self.yaw %= TAU;
		}


		let aspect = ctx.gfx.backbuffer_aspect();
		let projection = Mat4::perspective(PI/3.0, aspect, 0.01, 100.0)
			* Mat4::translate(Vec3::from_z(-3.0))
			* Mat4::rotate_x(PI/16.0)
			* Mat4::rotate_y(self.yaw);

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

		ctx.gfx.frame_encoder.command_group("Generate Geo")
			.compute(self.c_shader)
			.ssbo(0, self.vertex_buffer)
			.ssbo(1, self.line_index_buffer)
			.indirect(&[1u32, 1, 1]);

		ctx.gfx.frame_encoder.command_group("Rotate image colours")
			.compute(self.image_shader)
			.groups([3, 3, 1])
			.image_rw(0, self.image);

		let mut group = ctx.gfx.frame_encoder.command_group("Draw everything");
		group.bind_shared_ubo(2, self.vertex_buffer);
		group.bind_shared_sampled_image(0, self.image, self.sampler);

		self.time += 1.0/60.0;

		let upload_id = group.upload(&[self.time]);
		
		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::PrimitiveType::Triangles)
			.elements(3)
			.ubo(0, &[0.0f32]);
		
		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::PrimitiveType::Triangles)
			.elements(3)
			.instances(8)
			.sampled_image(0, self.cool_image, self.sampler)
			.ubo(0, upload_id);

		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::PrimitiveType::Points)
			.elements(10)
			.ubo(0, &[self.time*2.0]);

		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::PrimitiveType::Lines)
			.indexed(self.line_index_buffer)
			.elements(6)
			.ubo(0, &[self.time/2.0]);

		if let Some(pos) = ctx.input.pointer_position() {
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
				.indexed(&[0u32, 2, 3, 2, 1, 3])
				.sampled_image(0, self.cool_image, self.sampler);
		}


		let up = Vec3::from_y(1.0);
		let right = Vec3::from_y_angle(self.yaw);

		self.sprites.basic(Vec3::from_z(-1.0), up, Vec3::zero(), Color::rgb(1.0, 0.0, 1.0));

		self.sprites.basic(right * 0.5, up * 0.5, Vec3::from_y(1.0), Color::rgb(1.0, 0.5, 0.5));

		self.sprites.basic(right * 0.5, up, Vec3::new(1.5, 0.0, 1.0), Color::rgb(0.5, 1.0, 0.5));
		self.sprites.basic(right * 0.5, up, Vec3::new(-1.0, 0.0, 2.5), Color::rgb(0.5, 1.0, 0.5));
		self.sprites.basic(right * 0.5, up * 0.7, Vec3::new(3.0, 0.0, -1.5), Color::rgb(0.5, 1.0, 0.5));

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