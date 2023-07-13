use toybox::*;

fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_BACKTRACE", "1");

	toybox::run("playbox", |_| Ok(App))
}



struct App;

impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		ctx.gfx.core.push_debug_group("Wahoo");

		unsafe {
			ctx.gfx.core.gl.ClearColor(0.5, 0.5, 0.5, 1.0);
			// ctx.gfx.core.gl.Clear(gl::COLOR_BUFFER_BIT);

			ctx.gfx.core.gl.ClearNamedFramebufferfv(0, gl::COLOR, 0, [0.5, 0.0, 0.5, 1.0].as_ptr());
		}

		ctx.gfx.core.debug_marker("Hello");

		ctx.gfx.core.pop_debug_group();

		ctx.gfx.core.debug_marker("Goodbye");
	}
}