use toybox::*;

fn main() -> anyhow::Result<()> {
	// std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", App::new)
}



struct App {
	v_shader: gfx::resource_manager::shader::ShaderHandle,
	f_shader: gfx::resource_manager::shader::ShaderHandle,

	pipeline: gfx::core::ShaderPipelineName,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.gfx.frame_encoder.backbuffer_color([1.0, 0.5, 1.0]);

		let mut group = ctx.gfx.frame_encoder.command_group("START");
		group.debug_marker("FUCK");

		unsafe {
			let mut vao = 0;
			ctx.gfx.core.gl.CreateVertexArrays(1, &mut vao);
			ctx.gfx.core.gl.BindVertexArray(vao);
		}

		use gfx::resource_manager::shader::ShaderDef;

		Ok(App {
			v_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.vs.glsl")?),
			f_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.fs.glsl")?),

			pipeline: ctx.gfx.core.create_shader_pipeline(),
		})
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		let mut group = ctx.gfx.frame_encoder.command_group("MY Group");
		group.debug_marker("Group Time");

		let pipeline = self.pipeline;
		let v_shader = self.v_shader;
		let f_shader = self.f_shader;
		group.execute(move |core, rm| {
			core.attach_shader_to_pipeline(pipeline, rm.shaders.get_name(v_shader).unwrap());
			core.attach_shader_to_pipeline(pipeline, rm.shaders.get_name(f_shader).unwrap());

			unsafe {
				core.gl.BindProgramPipeline(pipeline.0);
				core.gl.DrawArrays(gl::POINTS, 0, 10);
			}

			core.debug_marker("User Callback");
		});
	}
}