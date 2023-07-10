use toybox::*;

fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", |_| Ok(App))
}



struct App;

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		unsafe {
			ctx.gfx_core.gl.ClearColor(0.5, 0.5, 0.5, 1.0);
			ctx.gfx_core.gl.Clear(gl::COLOR_BUFFER_BIT);
		}
	}
}