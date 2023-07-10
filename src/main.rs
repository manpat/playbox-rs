use toybox::prelude::*;

use toybox::host::Host;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let host = Host::create()?;
	let Host{ event_loop, gl_state: gl, surface, gl_context, .. } = host;

	
	event_loop.run(move |event, _, control_flow| {
		use host::winit::event::*;
		use host::glutin::surface::GlSurface;

		control_flow.set_wait();

		match event {
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => control_flow.set_exit(),


			Event::DeviceEvent { event: DeviceEvent::Key(KeyboardInput{ virtual_keycode: Some(VirtualKeyCode::Escape), .. }), .. } => {
				control_flow.set_exit();
			}

			Event::MainEventsCleared => unsafe {
				gl.ClearColor(1.0, 0.0, 1.0, 1.0);
				gl.Clear(gl::COLOR_BUFFER_BIT);


				surface.swap_buffers(&gl_context).unwrap();
			}


			_ => (),
		}
	})

	// let mut engine = toybox::Engine::new("playbox")?;

	// 'main: loop {
	// 	engine.process_events();
	// 	if engine.should_quit() || engine.input.raw_state.active_buttons.contains(&input::Keycode::Escape.into()) {
	// 		break 'main
	// 	}

	// 	engine.end_frame();
	// }
}

