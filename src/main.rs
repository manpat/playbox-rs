use toybox::prelude::*;

use toybox::host::Host;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let host = Host::create("playbox")?;
	host.install_default_error_handler();

	let Host{ event_loop, gl_state: gl, surface, gl_context, .. } = host;
	let gfx_core = gfx::Core::new(surface, gl_context, gl);

	
	event_loop.run(move |event, _, control_flow| {
		use host::winit::event::*;

		control_flow.set_poll();

		match event {
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => control_flow.set_exit(),

			Event::DeviceEvent { event: DeviceEvent::Key(KeyboardInput{ virtual_keycode: Some(VirtualKeyCode::Escape), .. }), .. } => {
				control_flow.set_exit();
			}

			Event::MainEventsCleared => unsafe {
				gfx_core.gl.ClearColor(0.5, 0.5, 0.5, 1.0);
				gfx_core.gl.Clear(gl::COLOR_BUFFER_BIT);


				gfx_core.finalize_frame();
			}

			// Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
			// 	main_loop.resize(Vec2i::new(physical_size.width as i32, physical_size.height as i32));
			// }


			_ => (),
		}
	})
}

