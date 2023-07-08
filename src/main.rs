use toybox::prelude::*;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	'main: loop {
		engine.process_events();
		if engine.should_quit() || engine.input.raw_state.active_buttons.contains(&input::Keycode::Escape.into()) {
			break 'main
		}

		engine.end_frame();
	}

	Ok(())
}

