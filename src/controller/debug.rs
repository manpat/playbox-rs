use toybox::prelude::*;
use toybox::input::{raw::Scancode, raw::MouseButton};

use crate::model;

toybox::declare_input_context! {
	struct Actions "Debug" {
		trigger toggle_active { "Toggle" [Scancode::Grave] }
		trigger toggle_flycam { "Toggle Fly Cam" [Scancode::V] }
	}
}

toybox::declare_input_context! {
	struct ActiveActions "Active Debug" {
		trigger reset_gems { "Reset Gems" [Scancode::F1] }
		trigger dump_stats { "Dump Perf Stats" [Scancode::F12] }

		state left_mouse { "Interact" [MouseButton::Left] }
		pointer mouse { "Mouse" }
	}
}


pub struct DebugController {
	actions: Actions,
	active_actions: ActiveActions,
}

impl DebugController {
	pub fn new(engine: &mut toybox::Engine) -> DebugController {
		let actions = Actions::new(&mut engine.input);
		let active_actions = ActiveActions::new(&mut engine.input);

		engine.input.enter_context(actions.context_id());

		DebugController {
			actions, active_actions
		}
	}

	pub fn update(&self, engine: &mut toybox::Engine, debug_model: &mut model::Debug, scene: &mut model::Scene,
		camera: &mut model::Camera)
	{
		let currently_active = engine.input.is_context_active(self.active_actions.context_id());

		if engine.input.frame_state().active(self.actions.toggle_active) {
			if currently_active {
				engine.input.leave_context(self.active_actions.context_id());
			} else {
				engine.input.enter_context(self.active_actions.context_id());
			}

			debug_model.active = !currently_active;
		}

		let input_state = engine.input.frame_state();

		if let Some(pos) = input_state.mouse(self.active_actions.mouse) {
			debug_model.mouse_pos = pos;
		}

		if input_state.active(self.active_actions.reset_gems) {
			for gem in scene.gems.iter_mut() {
				gem.state = model::scene::GemState::Idle;
			}
		}

		if input_state.active(self.actions.toggle_flycam) {
			use model::camera::ControlMode;

			camera.control_mode = match camera.control_mode {
				ControlMode::OrbitPlayer => ControlMode::FreeFly,
				ControlMode::FreeFly => ControlMode::OrbitPlayer,
			};
		}

		if input_state.active(self.active_actions.dump_stats) {
			if let Some(summary) = engine.instrumenter.summary() {
				println!("{:#?}", summary);
			}
		}
	}
}