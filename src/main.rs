use toybox::*;

fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", App::new)
}



struct App;

impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		ctx.gfx.frame_encoder.backbuffer_color([1.0, 0.5, 1.0]);

		let mut group = ctx.gfx.frame_encoder.command_group("START");
		group.debug_marker("FUCK");

		Ok(App)
	}
}

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		let mut group = ctx.gfx.frame_encoder.command_group("MY Group");
		group.debug_marker("Group Time");
		group.execute(|core, _rm| {
			core.debug_marker("User Callback");
		});
	}
}