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
		buffer: vec![0.0; 1<<13].into_boxed_slice().into(),
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
			draw(ui, audio, mixer_id, &buffer);
		}

		engine = next_frame(engine).await;
	}

	Ok(())
}


fn draw(ui: &imgui::Ui<'static>, audio: &mut audio::AudioSystem, mixer_id: audio::NodeId, buffer: &VisualiserBuffer) {
	use audio::node_builder::*;

	if ui.button("Play") {
		audio.queue_update(move |graph| {
			let noise = NoiseGenerator::new().envelope(0.01, 0.2);
			let osc = OscillatorGenerator::new(110.0).envelope(0.03, 0.5);
			let node = (noise, osc).low_pass(200.0).build();
			graph.add_node(node, mixer_id);
		});
	}

	ui.same_line();

	if ui.button("Play 2") {
		audio.queue_update(move |graph| {
			let noise = NoiseGenerator::new().envelope(0.3, 0.8);
			let osc1 = OscillatorGenerator::new(55.0).envelope(0.2, 0.5);
			let osc2 = OscillatorGenerator::new(220.0).envelope(0.03, 0.5);
			let node = (noise, osc1, osc2).low_pass(200.0).build();
			graph.add_node(node, mixer_id);
		});
	}

	ui.plot_lines("##Samples", unsafe{buffer.get()})
		.scale_min(-1.0)
		.scale_max(1.0)
		.graph_size([0.0, 300.0])
		.build();
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