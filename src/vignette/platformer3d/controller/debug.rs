use toybox::prelude::*;
use toybox::input::{raw::Scancode, raw::MouseButton};

use crate::vignette::platformer3d::model;

toybox::declare_input_context! {
	struct Actions "Debug" {
		priority [10]

		trigger toggle_active { "Toggle" [Scancode::Grave] }
		trigger toggle_flycam { "Toggle Fly Cam" [Scancode::V] }
	}
}

toybox::declare_input_context! {
	struct ActiveActions "Active Debug" {
		priority [10]

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
		DebugController {
			actions: Actions::new_active(engine),
			active_actions: ActiveActions::new(engine),
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
			reset_gems(scene);
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

		let ui = engine.imgui.frame();

		if let Some(_) = imgui::Window::new("Debug").begin(ui)
		{
			ui.checkbox("Srgb Test", &mut debug_model.srgb_active);
			ui.checkbox("Perf View", &mut debug_model.perf_active);

			if ui.button("Reset Gems") {
				reset_gems(scene);
			}

			let camera_mode = &mut camera.control_mode;
			if let Some(_) = ui.begin_combo("Camera Mode", format!("{:?}", camera_mode)) {
				use model::camera::ControlMode;

				if imgui::Selectable::new("OrbitPlayer").build(ui) {
					*camera_mode = ControlMode::OrbitPlayer;
				}

				if imgui::Selectable::new("FreeFly").build(ui) {
					*camera_mode = ControlMode::FreeFly;
				}
			}
		}

		if debug_model.perf_active {
			if let Some(_window) = imgui::Window::new("Perf")
				.opened(&mut debug_model.perf_active)
				.begin(ui)
			{
				if let Some(summary) = engine.instrumenter.summary() {
					ui.label_text("Total Triangles", summary.total_triangles.to_string());
					ui.label_text("Total GPU ms", format!("{:.2}ms", summary.total_gpu_time_ms));
					ui.label_text("Total CPU ms", format!("{:.2}ms", summary.total_cpu_time_ms));

					for section in summary.sections.iter() {
						if let Some(_node) = imgui::TreeNode::new(&section.name)
							.default_open(true)
							.push(ui)
						{
							ui.label_text("Triangles", section.triangles.to_string());
							ui.label_text("GPU time", format!("{:.2}ms", section.gpu_time_ms));
							ui.label_text("CPU time", format!("{:.2}ms", section.cpu_time_ms));
						}
					}
				}
			}
		}
	}
}


fn reset_gems(scene: &mut model::Scene) {
	for gem in scene.gems.iter_mut() {
		gem.state = model::scene::GemState::Idle;
	}
}