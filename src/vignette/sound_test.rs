use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::cell::UnsafeCell;


mod simple;
mod sequencer;


pub async fn play() -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = crate::global_controller::GlobalController::new(&mut engine, resource_scope_token.id())?;

	let buffer = VisualiserBuffer {
		buffer: vec![0.0; 1<<16].into_boxed_slice().into(),
		cursor: AtomicUsize::new(0),
	};

	let buffer: Arc<VisualiserBuffer> = Arc::new(buffer);

	let global_audio_state = engine.audio.update_graph_immediate(|graph| {
		use audio::*;

		let mixer_node = nodes::MixerNode::new_stereo(1.0);
		let compressor_node = nodes::CompressorNode::new(0.02, 0.2, -8.0, -0.1);
		let vis_node = VisualiserNode { buffer: Arc::clone(&buffer) };

		// TODO(pat.m): the resonant filter seems kinda unstable at high cutoffs - I wonder if this is a precision issue
		// or just something inherent in the filter I've implemented
		let lpf_cutoff = AtomicFloatParameter::new(12000.0);
		let lpf_q = AtomicFloatParameter::new(0.0);
		let lpf_node = StereoEffectNode::new(effect::ResonantLowPass::new(lpf_cutoff.clone(), lpf_q.clone()));

		let vis_id = graph.add_node(vis_node, None);
		let lpf_id = graph.add_node(lpf_node, None);
		let compress_id = graph.add_node(compressor_node, None);
		let mixer_id = graph.add_node(mixer_node, None);

		graph.add_send_chain(&[mixer_id, lpf_id, compress_id, vis_id, graph.output_node()]);

		// Without this the mixer is freed
		graph.pin_node_to_scope(mixer_id, &resource_scope_token);

		GlobalAudioState {
			mixer_id,
			lpf_cutoff,
			lpf_q,
		}
	});

	let mut simple_panel = simple::SimplePanel::new(global_audio_state.clone());
	let mut seq_panel = sequencer::SequencerPanel::new(global_audio_state.clone());


	let mut vis_buffer = vec![0.0; 1<<14];
	let mut freeze_vis = false;

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
			let window_width = ui.window_size()[0];

			let read_buffer = unsafe{buffer.get()};
			if !freeze_vis {
				let read_end = buffer.cursor.load(Ordering::Relaxed);
				let buffer_size = read_buffer.len();
				let vis_buffer_size = vis_buffer.len();

				if read_end > vis_buffer_size {
					let read_start = read_end - vis_buffer_size;
					vis_buffer.copy_from_slice(&read_buffer[read_start..read_end]);
				} else {
					let write_split = vis_buffer_size - read_end;
					let read_start = buffer_size - write_split;

					vis_buffer[..write_split].copy_from_slice(&read_buffer[read_start..]);
					vis_buffer[write_split..].copy_from_slice(&read_buffer[..read_end]);
				}
			}


			ui.plot_lines("##Samples", &vis_buffer)
				.scale_min(-1.0)
				.scale_max(1.0)
				.graph_size([window_width - 20.0, 300.0])
				.build();


			ui.checkbox("Freeze", &mut freeze_vis);

			ui.same_line();

			let mut vis_buffer_size = vis_buffer.len() as u32;
			if imgui::Slider::new("Samples", 100, read_buffer.len() as u32)
				.build(ui, &mut vis_buffer_size)
			{
				vis_buffer.resize(vis_buffer_size as usize, 0.0);
			}


			let mut current_cutoff = global_audio_state.lpf_cutoff.read();
			if imgui::Slider::new("Global LPF", 1.0, 16000.0)
				.flags(imgui::SliderFlags::LOGARITHMIC)
				.build(ui, &mut current_cutoff)
			{
				global_audio_state.lpf_cutoff.write(current_cutoff.max(0.0));
			}


			let mut current_q = global_audio_state.lpf_q.read();
			if imgui::Slider::new("Global Q", 0.0, 1.0)
				.build(ui, &mut current_q)
			{
				global_audio_state.lpf_q.write(current_q.clamp(0.0, 1.0));
			}



			if let Some(_tab_bar) = ui.tab_bar("Panel Tabs") {
				if let Some(_item) = ui.tab_item("Simple") {
					simple_panel.update(audio, ui);
				}
				if let Some(_item) = ui.tab_item("Sequencer") {
					seq_panel.update(audio, ui);
				}
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




#[derive(Clone)]
pub struct GlobalAudioState {
	pub mixer_id: audio::NodeId,

	pub lpf_cutoff: audio::AtomicFloatParameter,
	pub lpf_q: audio::AtomicFloatParameter,
}

