use toybox::prelude::*;
use toybox::input::raw::Scancode;
use toybox::utility::ResourceScopeID;


toybox::declare_input_context! {
	struct GlobalActions "Global" {
		trigger quit { "Quit" [Scancode::Escape] }
		trigger toggle_wireframe { "Toggle Wireframe" [Scancode::Z] }
	}
}


pub struct GlobalController {
	actions: GlobalActions,

	should_quit: bool,
	wireframe_enabled: bool,
}

impl GlobalController {
	pub fn new(engine: &mut toybox::Engine, _: ResourceScopeID) -> Result<GlobalController, Box<dyn Error>> {
		Ok(GlobalController {
			actions: GlobalActions::new_active(engine),

			should_quit: false,
			wireframe_enabled: false,
		})
	}

	pub fn update(&mut self, engine: &mut toybox::Engine) {
		let input_state = engine.input.frame_state();

		if input_state.active(self.actions.quit) {
			self.should_quit = true
		}

		if input_state.active(self.actions.toggle_wireframe) {
			self.wireframe_enabled = !self.wireframe_enabled;
			engine.gfx.draw_context().set_wireframe(self.wireframe_enabled);
		}
	}

	pub fn should_quit(&self) -> bool {
		self.should_quit
	}
}
