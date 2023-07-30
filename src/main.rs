use toybox::*;

fn main() -> anyhow::Result<()> {
	// std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", App::new)
}



struct App {
	v_shader: gfx::resource_manager::shader::ShaderHandle,
	f_shader: gfx::resource_manager::shader::ShaderHandle,
}

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.gfx.frame_encoder.backbuffer_color([1.0, 0.5, 1.0]);

		let mut group = ctx.gfx.frame_encoder.command_group("START");
		group.debug_marker("FUCK");

		use gfx::resource_manager::shader::ShaderDef;

		Ok(App {
			v_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.vs.glsl")?),
			f_shader: ctx.gfx.resource_manager.create_shader(ShaderDef::from("shaders/test.fs.glsl")?),
		})
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		let mut group = ctx.gfx.frame_encoder.command_group("MY Group");
		group.debug_marker("Group Time");

		group.upload_heap.stage_data(&[1.0f32, 2.0, 3.0]);
		let upload_id = group.upload_heap.stage_data(&[1u32]);
		group.upload_heap.stage_data(&[1.0f32, 2.0, 3.0]);
		
		group.upload_heap.update_staged_upload_alignment(upload_id, 128);

		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Triangles)
			.elements(3)
			.instances(8);

		group.draw(self.v_shader, self.f_shader)
			.primitive(gfx::command::draw::PrimitiveType::Points)
			.elements(10);

	}
}