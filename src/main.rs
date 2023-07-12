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

			let message = b"Hello\0";
			let id = 1234;

			ctx.gfx_core.gl.DebugMessageInsert(
				gl::DEBUG_SOURCE_APPLICATION,
				gl::DEBUG_TYPE_MARKER,
				id,
				gl::DEBUG_SEVERITY_NOTIFICATION,
				message.len() as i32,
				message.as_ptr() as *const _
			);

			let message = b"Goodbye\0";

			ctx.gfx_core.gl.DebugMessageInsert(
				gl::DEBUG_SOURCE_APPLICATION,
				gl::DEBUG_TYPE_MARKER,
				id,
				gl::DEBUG_SEVERITY_HIGH,
				message.len() as i32,
				message.as_ptr() as *const _
			);
		}
	}
}