use toybox::prelude::*;
use toybox::input::{InputSystem, raw::Scancode, raw::MouseButton};

use crate::model;

toybox::declare_input_context! {
	struct Actions "Debug" {
		trigger toggle_active { "Toggle" [Scancode::Grave] }
	}
}

toybox::declare_input_context! {
	struct ActiveActions "Active Debug" {
		state left_mouse { "Interact" [MouseButton::Left] }
		pointer mouse { "Mouse" }
	}
}


pub struct DebugController {
	actions: Actions,
	active_actions: ActiveActions,
}

impl DebugController {
	pub fn new(input: &mut InputSystem) -> DebugController {
		let actions = Actions::new(input);
		let active_actions = ActiveActions::new(input);

		input.enter_context(actions.context_id());

		DebugController {
			actions, active_actions
		}
	}

	pub fn update(&self, input: &mut InputSystem, model: &mut model::Debug) {
		let currently_active = input.is_context_active(self.active_actions.context_id());

		if input.frame_state().active(self.actions.toggle_active) {
			if currently_active {
				input.leave_context(self.active_actions.context_id());
			} else {
				input.enter_context(self.active_actions.context_id());
			}

			model.active = !currently_active;
		}

		if let Some(pos) = input.frame_state().mouse(self.active_actions.mouse) {
			model.mouse_pos = pos;
		}
	}
}