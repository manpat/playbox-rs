use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::cell::UnsafeCell;


pub async fn play() -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = crate::global_controller::GlobalController::new(&mut engine, resource_scope_token.id())?;

	let buffer = VisualiserBuffer {
		buffer: vec![0.0; 1<<11].into_boxed_slice().into(),
		cursor: AtomicUsize::new(0),
	};

	let buffer: Arc<VisualiserBuffer> = Arc::new(buffer);

	let mixer_id = engine.audio.update_graph_immediate(|graph| {
		let mixer_node = audio::nodes::MixerNode::new_stereo(1.0);
		let vis_node = VisualiserNode { buffer: Arc::clone(&buffer) };
		let vis_id = graph.add_node(vis_node, graph.output_node());
		let mixer_id = graph.add_node(mixer_node, vis_id);

		// Without this the mixer is freed
		graph.pin_node_to_scope(mixer_id, &resource_scope_token);
		mixer_id
	});

	let mut pulse_width = 0.25;
	let mut base_frequency = 220.0;

	let mut env_attack = 0.03;
	let mut env_release = 1.5;

	let mut env_exp_attack = 1.0 / 4.0;
	let mut env_exp_release = 4.0;

	'main: loop {
		global_controller.update(&mut engine);
		if global_controller.should_quit() {
			break 'main
		}

		let toybox::Engine{ imgui, audio, gfx, .. } = &mut *engine;

		imgui.set_input_enabled(true);
		imgui.set_visible(true);


		let mut gfx = gfx.draw_context();
		gfx.set_clear_color(Color::grey(0.1));
		gfx.clear(gfx::ClearMode::ALL);

		let size = gfx.backbuffer_size().to_vec2().to_array();


		let ui = imgui.frame();

		if let Some(_window) = imgui::Window::new("Sound Test")
			.size(size, imgui::Condition::Always)
			.position([0.0; 2], imgui::Condition::Always)
			.resizable(false)
			.movable(false)
			.collapsible(false)
			.begin(ui)
		{
			use audio::node_builder::*;
			use audio::generator as gen;
			use audio::envelope as env;

			let window_width = ui.window_size()[0];

			ui.plot_lines("##Samples", unsafe{buffer.get()})
				.scale_min(-1.0)
				.scale_max(1.0)
				.graph_size([window_width - 20.0, 300.0])
				.build();

			ui.columns(2, "Controls", true);

			{
				if ui.button("Play") {
					audio.queue_update(move |graph| {
						let noise = NoiseGenerator::new().envelope(env::AR::new(0.01, 0.2).exp(4.0));
						let osc = gen::GeneratorNode::new_sine(base_frequency).envelope(env::AR::new(0.03, 0.5).exp(4.0));
						let node = (noise, osc).low_pass(200.0).build();
						graph.add_node(node, mixer_id);
					});
				}

				ui.same_line();

				if ui.button("Play 2") {
					audio.queue_update(move |graph| {
						let noise = NoiseGenerator::new().envelope(env::AR::new(0.3, 1.5).exp(4.0));
						let osc1 = gen::GeneratorNode::new_triangle(base_frequency / 2.0).envelope(env::AR::new(0.2, 0.5).exp(4.0));
						let osc2 = gen::GeneratorNode::new_pulse(base_frequency, 0.1).envelope(env::AR::new(0.03, 2.0).exp(4.0));
						let node = (noise, osc1, osc2).low_pass(200.0).high_pass(2.0).build();
						graph.add_node(node, mixer_id);
					});
				}

				ui.same_line();

				if ui.button("Play 3") {
					audio.queue_update(move |graph| {
						let node = gen::GeneratorNode::new(base_frequency, |p| gen::pulse_wave(p, 0.1))
							.envelope(env::AR::new(env_attack, env_release).exp2(env_exp_attack, env_exp_release))
							.high_pass(10.0)
							.build();
						graph.add_node(node, mixer_id);
					});
				}

				if ui.button("Sine") {
					audio.queue_update(move |graph| {
						let node = gen::GeneratorNode::new_sine(base_frequency)
							.envelope(env::AR::new(env_attack, env_release).exp2(env_exp_attack, env_exp_release))
							.build();
						graph.add_node(node, mixer_id);
					});
				}

				if ui.button("Triangle") {
					audio.queue_update(move |graph| {
						let node = gen::GeneratorNode::new_triangle(base_frequency)
							.envelope(env::AR::new(env_attack, env_release).exp2(env_exp_attack, env_exp_release))
							.build();
						graph.add_node(node, mixer_id);
					});
				}

				if ui.button("Square") {
					audio.queue_update(move |graph| {
						let node = gen::GeneratorNode::new_square(base_frequency)
							.envelope(env::AR::new(env_attack, env_release).exp2(env_exp_attack, env_exp_release))
							.build();
						graph.add_node(node, mixer_id);
					});
				}

				if ui.button("Saw") {
					audio.queue_update(move |graph| {
						let node = gen::GeneratorNode::new_saw(base_frequency)
							.envelope(env::AR::new(env_attack, env_release).exp2(env_exp_attack, env_exp_release))
							.build();
						graph.add_node(node, mixer_id);
					});
				}

				if ui.button("Pulse") {
					audio.queue_update(move |graph| {
						let node = gen::GeneratorNode::new(base_frequency, move |p| gen::pulse_wave(p, pulse_width))
							.envelope(env::AR::new(env_attack, env_release).exp2(env_exp_attack, env_exp_release))
							.high_pass(1.0)
							.build();
						graph.add_node(node, mixer_id);
					});
				}

				ui.same_line();

				imgui::Slider::new("Width", 0.0, 1.0)
					.build(ui, &mut pulse_width);
			}

			ui.next_column();

			
			let midi_note_f = ((base_frequency as f64/ 440.0).log2() * 12.0 + 69.0);
			let mut midi_note = midi_note_f.trunc() as i32;
			let mut cents = (midi_note_f.fract() * 100.0) as i32;

			{
				imgui::Slider::new("Frequency", 22.0, 880.0)
					.flags(imgui::SliderFlags::LOGARITHMIC)
					.build(ui, &mut base_frequency);


				if imgui::Slider::new("Midi Note", 16, 81)
					.build(ui, &mut midi_note)
				{
					base_frequency = 440.0 * ((midi_note as f32 - 69.0 + cents as f32/100.0)/12.0).exp2() as f32;
				}

				if imgui::Slider::new("Cents", 0, 99)
					.build(ui, &mut cents)
				{
					base_frequency = 440.0 * ((midi_note as f32 - 69.0 + cents as f32/100.0)/12.0).exp2() as f32;
				}

				if ui.button("Sync To Oscilloscope") {
					base_frequency = 44100.0 / 256.0;
				}

				ui.same_line();
				if ui.button("+ Octave") {
					base_frequency *= 2.0;
				}

				ui.same_line();
				if ui.button("- Octave") {
					base_frequency /= 2.0;
				}

				ui.new_line();

				imgui::Slider::new("Attack", 0.01, 4.0)
					.build(ui, &mut env_attack);

				imgui::Slider::new("Attack Curve", 0.01, 4.0)
					.flags(imgui::SliderFlags::LOGARITHMIC)
					.build(ui, &mut env_exp_attack);

				imgui::Slider::new("Release", 0.01, 4.0)
					.build(ui, &mut env_release);

				imgui::Slider::new("Release Curve", 0.01, 4.0)
					.flags(imgui::SliderFlags::LOGARITHMIC)
					.build(ui, &mut env_exp_release);
			}


			ui.columns(1, "##Stop it", false);
			ui.separator();

			let octave = midi_note / 12 - 1; // C0 is 12
			let note_name = match midi_note % 12 {
				0 => "C",
				1 => "C#",
				2 => "D",
				3 => "D#",
				4 => "E",
				5 => "F",
				6 => "F#",
				7 => "G",
				8 => "G#",
				9 => "A",
				10 => "A#",
				11 => "B",
				_ => "????"
			};

			let midi_note_diff = (midi_note_f.fract() * 100.0) as i32;

			if midi_note_diff != 0 {
				ui.label_text("Note", format!("{note_name}{octave} +{cents}cents"));
			} else {
				ui.label_text("Note", format!("{note_name}{octave}"));
			}
		}

		engine = next_frame(engine).await;
	}

	Ok(())
}





struct VisualiserBuffer {
	buffer: UnsafeCell<Box<[f32]>>,
	cursor: AtomicUsize,
}

impl VisualiserBuffer {
	unsafe fn get(&self) -> &[f32] {
		unsafe {
			&*self.buffer.get()
		}
	}

	unsafe fn get_mut(&self) -> &mut [f32] {
		unsafe {
			&mut *self.buffer.get()
		}
	}
}

unsafe impl Sync for VisualiserBuffer {}


use audio::{EvaluationContext, ProcessContext};

struct VisualiserNode {
	buffer: Arc<VisualiserBuffer>,
}

impl audio::Node for VisualiserNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }
	fn node_type(&self, _: &EvaluationContext<'_>) -> audio::NodeType { audio::NodeType::Effect }

	fn process(&mut self, ProcessContext{ inputs, output, .. }: ProcessContext<'_>) {
		let Some(input) = inputs.first() else {
			output.fill(0.0);
			return
		};

		output.as_simd_mut().copy_from_slice(input.as_simd());

		let left_samples_wide = input.as_simd().array_chunks::<2>()
			.map(|&[a, b]| {
				let (l, _r) = a.deinterleave(b);
				l
			});

		// SAFETY: Its not safe lmao. this is UB, but reading/writing floats is already atomic in x86
		// and we can be reasonably sure that rust won't optimise these writes _away_.
		// The Good™ way to do this would probably be with raw pointer writes.
		let target_slice = unsafe { self.buffer.get_mut() };

		for samples_wide in left_samples_wide {
			let lanes = samples_wide.lanes();
			let old_cursor = self.buffer.cursor.fetch_add(lanes, Ordering::Relaxed);
			assert!(old_cursor % lanes == 0);

			let real_cursor = old_cursor % target_slice.len();

			target_slice[real_cursor..real_cursor + lanes].copy_from_slice(&samples_wide.to_array());
		}

		// Wrap the cursor
		let current_cursor = self.buffer.cursor.load(Ordering::Relaxed);
		self.buffer.cursor.store(current_cursor % target_slice.len(), Ordering::Relaxed);
	}
}