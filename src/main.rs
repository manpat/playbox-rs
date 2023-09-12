use toybox::*;

fn main() -> anyhow::Result<()> {
	// std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", App::new)
}



struct App {
	v_shader: gfx::resource_manager::shader::ShaderHandle,
	f_shader: gfx::resource_manager::shader::ShaderHandle,
	c_shader: gfx::resource_manager::shader::ShaderHandle,

	vertex_buffer: gfx::core::BufferName,

	time: f32,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.gfx.frame_encoder.backbuffer_color([1.0, 0.5, 1.0]);

		// ctx.audio.set_provider(MyAudioProvider::default())?;

		dbg!(&ctx.gfx.core.capabilities());

		let mut group = ctx.gfx.frame_encoder.command_group("START");
		group.debug_marker("FUCK");

		use gfx::resource_manager::shader::ShaderDef;

		Ok(App {
			v_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.vs.glsl")?),
			f_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.fs.glsl")?),
			c_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.cs.glsl")?),

			vertex_buffer: {
				let buffer = ctx.gfx.core.create_buffer();
				let flags = 0; // not client visible
				// TODO(pat.m): using Vec4 here to cope with std140 layout. this is an easy mistake to make
				// how to make this better?
				ctx.gfx.core.allocate_buffer_storage(buffer, std::mem::size_of::<[Vec4; 3]>(), flags);
				buffer
			},

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

		use gfx::bindings::BufferBindTargetDesc;
		ctx.gfx.frame_encoder.global_bindings.bind_buffer(BufferBindTargetDesc::UboIndex(1), projection_upload);

		let mut group = ctx.gfx.frame_encoder.command_group("Compute time");
		group.compute(self.c_shader)
			.ssbo(0, self.vertex_buffer);

		group.execute(move |core, _rm| {
			unsafe {
				core.gl.MemoryBarrier(gl::UNIFORM_BARRIER_BIT);
			}
		});

		let mut group = ctx.gfx.frame_encoder.command_group("MY Group");
		group.debug_marker("Group Time");
		group.ubo(2, self.vertex_buffer);

		self.time += 1.0/60.0;

		let upload_id = group.upload(&self.time);
		
		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Triangles)
			.elements(3)
			.ubo(0, &0.0f32);
		
		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Triangles)
			.elements(3)
			.instances(8)
			.ubo(0, upload_id);

		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Points)
			.elements(10)
			.ubo(0, &(self.time*2.0));

		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Lines)
			.indexed(&[0u32, 1, 1, 2, 2, 3])
			.elements(6)
			.ubo(0, &1.0f32);

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