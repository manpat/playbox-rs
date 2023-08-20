use toybox::*;

fn main() -> anyhow::Result<()> {
	// std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", App::new)
}



struct App {
	v_shader: gfx::resource_manager::shader::ShaderHandle,
	f_shader: gfx::resource_manager::shader::ShaderHandle,

	time: f32,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.gfx.frame_encoder.backbuffer_color([1.0, 0.5, 1.0]);

		ctx.audio.set_provider(MyAudioProvider::default())?;

		let mut group = ctx.gfx.frame_encoder.command_group("START");
		group.debug_marker("FUCK");

		use gfx::resource_manager::shader::ShaderDef;

		Ok(App {
			v_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.vs.glsl")?),
			f_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.fs.glsl")?),

			time: 0.0,
		})
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		let aspect = ctx.gfx.backbuffer_aspect();
		let projection = Mat4::perspective(PI/3.0, aspect, 0.01, 100.0)
			* Mat4::translate(Vec3::from_z(-3.0))
			* Mat4::rotate_x(PI/16.0)
			* Mat4::rotate_y(PI/6.0 + self.time/3.0);

		let projection_upload = ctx.gfx.frame_encoder.upload(&projection);

		let mut group = ctx.gfx.frame_encoder.command_group("MY Group");
		group.debug_marker("Group Time");

		self.time += 1.0/60.0;

		let upload_id = group.upload(&self.time);
		
		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Triangles)
			.elements(3)
			.ubo(0, &0.0f32)
			.ubo(1, projection_upload);
		
		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Triangles)
			.elements(3)
			.instances(8)
			.ubo(0, upload_id)
			.ubo(1, projection_upload);

		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Points)
			.elements(10)
			.ubo(0, &(self.time*2.0))
			.ubo(1, projection_upload);

	}
}


#[derive(Default)]
struct MyAudioProvider {
	sample_dt: f32,
	phase: f32,
}

impl audio::Provider for MyAudioProvider {
	fn on_configuration_changed(&mut self, config: audio::Configuration) {
		self.sample_dt = 1.0/config.sample_rate;
		assert!(config.channels == 2);
	}

	fn fill_buffer(&mut self, buffer: &mut [f32]) {
		for frame in buffer.chunks_exact_mut(2) {
			let osc = (self.phase * 110.0 * TAU).sin() * 0.1;
			let amp = 0.5 - self.phase.cos() * 0.5;
			let amp = amp * amp;

			frame[0] = osc * amp;
			frame[1] = -osc * amp;
			self.phase += self.sample_dt;
		}

		self.phase %= 2.0 * PI;
	}
}