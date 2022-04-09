use toybox::prelude::*;
use toybox::audio::nodes::*;

use crate::model;

use std::iter;


pub struct AudioTestController {
	plink_sound_key: audio::SoundId,
	plink_mixer_node: audio::NodeId,

	emitters: Vec<Emitter>,
}

impl AudioTestController {
	pub fn new(engine: &mut toybox::Engine, scene: &model::Scene) -> AudioTestController {
		// let buffer = (0..44100/4)
		// 	.map(|index| {
		// 		let t = index as f32 / 44100.0;
		// 		(880.0 * TAU * t).sin() * (1.0 - t*8.0).max(0.0).powi(4)
		// 	})
		// 	.collect();

		let buffer = super::load_audio_buffer("assets/chime.ogg").unwrap();

		let plink_sound_key = engine.audio.add_sound(buffer);
		let plink_mixer_node = engine.audio.add_node_with_send(MixerNode::new(0.1), engine.audio.output_node());


		// let drone_mixer_node = engine.audio.add_node_with_send(MixerNode::new_stereo(0.001), engine.audio.output_node());

		// engine.audio.update_graph(move |graph| {
		// 	let panner_node = graph.add_node(PannerNode::new(1.0), true);
		// 	let mixer_node = graph.add_node(MixerNode::new(2.0), true);

		// 	for freq in [55.0, 330.0] {
		// 		let node = graph.add_node(OscillatorNode::new(freq), false);
		// 		graph.add_send(node, mixer_node);
		// 	}

		// 	graph.add_send_chain(&[mixer_node, panner_node, drone_mixer_node]);
		// });

		// engine.audio.update_graph(move |graph| {
		// 	let panner_node = graph.add_node(PannerNode::new(-1.0), true);
		// 	let mixer_node = graph.add_node(MixerNode::new(1.0), true);

		// 	for freq in [220.0, 110.0, 550.0] {
		// 		let node = graph.add_node(OscillatorNode::new(freq), false);
		// 		graph.add_send(node, mixer_node);
		// 	}

		// 	graph.add_send_chain(&[mixer_node, panner_node, drone_mixer_node]);
		// });


		let emitters: Vec<_> = scene.main_scene().entities_with_prefix("SOUND_")
			.zip([1.0, 2.0/3.0, 3.0/4.0])
			.map(|(entity, ratio)| {
				let pan_parameter = FloatParameter::new(0.0);
				let attenuation_parameter = FloatParameter::new(0.0);
				let cutoff_parameter = FloatParameter::new(0.0);
				let freq_parameter = FloatParameter::new(110.0 * ratio);
				let feedback_parameter = FloatParameter::new(0.3);
				let position = entity.position;

				Emitter {
					pan_parameter,
					attenuation_parameter,
					cutoff_parameter,
					freq_parameter,
					feedback_parameter,

					position
				}
			})
			.collect();

		engine.audio.update_graph(|graph| {
			let mixer_node = graph.add_node(MixerNode::new_stereo(0.3), false);

			graph.add_send_chain(&[mixer_node, graph.output_node()]);

			for emitter in emitters.iter() {
				let pan_parameter = emitter.pan_parameter.clone();
				let attenuation_parameter = emitter.attenuation_parameter.clone();
				let cutoff_parameter = emitter.cutoff_parameter.clone();
				let freq_parameter = emitter.freq_parameter.clone();
				let feedback_parameter = emitter.feedback_parameter.clone();

				let osc_node = graph.add_node(EmitterNode::new(freq_parameter), false);

				let delay_line_node = graph.add_node(DelayLineNode::new(8000, feedback_parameter), true);

				let spatialise_node = SpatialiseNode {
					pan_parameter,
					attenuation_parameter,
					cutoff_parameter,

					filter: LowPass::new(),
				};

				let spatialise_node = graph.add_node(spatialise_node, true);

				graph.add_send_chain(&[osc_node, delay_line_node, spatialise_node, mixer_node]);
			}
		});

		AudioTestController {
			plink_sound_key,
			plink_mixer_node,

			emitters,
		}
	}

	pub fn update(&mut self, engine: &mut toybox::Engine, camera: &model::Camera) {
		let camera_orientation = Quat::from_yaw(camera.yaw);
		let camera_forward = camera_orientation.forward();
		let camera_right = camera_orientation.right();

		for Emitter { pan_parameter, attenuation_parameter, cutoff_parameter, position, .. } in self.emitters.iter() {
			let diff = *position - camera.position;
			let distance = diff.length();

			let saturation_zone = 1.0;
			let pan_reduction_zone = 2.0;
			let filter_reduction_zone = 4.0;

			let forwardness = camera_forward.dot(diff) / distance;
			let rightness = camera_right.dot(diff) / distance;
			let attenuation = (1.0 / (1.0 - saturation_zone + distance).max(1.0)).clamp(0.0, 1.0);

			// When camera is within saturation zone, reduce effect of panning
			let pan = rightness * ((distance - pan_reduction_zone) / pan_reduction_zone).clamp(0.0, 1.0);

			let forwardness_factor = forwardness * 0.5 + 0.5;
			let distance_factor = ((filter_reduction_zone - distance) / filter_reduction_zone).clamp(0.0, 1.0);

			let cutoff = 1.0 - (1.0 - forwardness_factor) * (1.0 - distance_factor.powi(2)*0.5);

			let cutoff = cutoff.powi(2).lerp(4000.0, 30000.0);

			pan_parameter.write(pan);
			attenuation_parameter.write(attenuation);
			cutoff_parameter.write(cutoff);
		}

		let ui = engine.imgui.frame();

		if let Some(_window) = imgui::Window::new("Audio").begin(ui) {
			if ui.button("Plink") {
				let node = SamplerNode::new(self.plink_sound_key);
				engine.audio.add_node_with_send(node, self.plink_mixer_node);
			}

			for emitter @ Emitter { position, .. } in self.emitters.iter() {
				let pan = emitter.pan_parameter.target();
				let attenuation = emitter.attenuation_parameter.target();
				let cutoff = emitter.cutoff_parameter.target();
				let mut freq = emitter.freq_parameter.target();
				let mut feedback = emitter.feedback_parameter.target();

				let pos_str = format!("{position:?}");
				let _id_scope = ui.push_id(&pos_str);

				ui.label_text("pos", pos_str);
				ui.label_text("pan", pan.to_string());
				ui.label_text("att", attenuation.to_string());
				ui.label_text("lpf", cutoff.to_string());

				if imgui::Slider::new("freq", 20.0, 880.0)
					.display_format("%.1fHz")
					.build(&ui, &mut freq)
				{
					emitter.freq_parameter.write(freq);
				}

				if imgui::Slider::new("fbk", -1.0, 1.0)
					.display_format("%.3f%")
					.build(&ui, &mut feedback)
				{
					emitter.feedback_parameter.write(feedback);
				}

				ui.separator();
			}
		}
	}
}


struct Emitter {
	pan_parameter: FloatParameter,
	attenuation_parameter: FloatParameter,
	cutoff_parameter: FloatParameter,
	freq_parameter: FloatParameter,

	feedback_parameter: FloatParameter,

	position: Vec3,
}




use std::sync::{Arc, atomic::AtomicU32, atomic::Ordering};
use audio::system::EvaluationContext;

#[derive(Clone, Debug)]
struct FloatParameter {
	target: Arc<AtomicU32>,
	prev: f32,
}

impl FloatParameter {
	fn new(initial_value: f32) -> FloatParameter {
		let encoded = initial_value.to_bits();
		FloatParameter {
			target: Arc::new(AtomicU32::new(encoded)),
			prev: initial_value,
		}
	}

	fn target(&self) -> f32 {
		f32::from_bits(self.target.load(Ordering::Relaxed))
	}

	fn read(&mut self, samples: usize) -> ParameterCurve {
		let target = self.target();
		let start = std::mem::replace(&mut self.prev, target);
		ParameterCurve {
			start,
			target,
			inc: (target - start) / samples as f32,
		}
	}

	fn write(&self, new_value: f32) {
		self.target.store(new_value.to_bits(), Ordering::Relaxed);
	}
}


struct ParameterCurve {
	start: f32,
	target: f32,
	inc: f32,
}

impl ParameterCurve {
	fn next(&mut self) -> f32 {
		let next = self.start + self.inc;

		let next = if self.inc >= 0.0 {
			next.min(self.target)
		} else {
			next.max(self.target)
		};

		std::mem::replace(&mut self.start, next)
	}
}



struct SpatialiseNode {
	pan_parameter: FloatParameter,
	attenuation_parameter: FloatParameter,
	cutoff_parameter: FloatParameter,

	filter: LowPass,
}


impl Node for SpatialiseNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }

	fn process(&mut self, ProcessContext{inputs, output, eval_ctx}: ProcessContext<'_>) {
		assert!(inputs.len() == 1);
		assert!(output.stereo());

		let input = &inputs[0];
		assert!(!input.stereo());

		let smoothing = input.len();
		let mut pan_curve = self.pan_parameter.read(smoothing);
		let mut attenuation_curve = self.attenuation_parameter.read(smoothing);
		let mut cutoff_curve = self.cutoff_parameter.read(smoothing);

		let dt = 1.0 / eval_ctx.sample_rate;

		for ([out_l, out_r], &in_sample) in output.array_chunks_mut::<2>().zip(input.iter()) {
			let cutoff = cutoff_curve.next().max(1.0);

			let filtered_sample = self.filter.next(in_sample, cutoff, dt);

			let pan_value = pan_curve.next().clamp(-1.0, 1.0) / 2.0 + 0.5;

			let r_pan = (pan_value).sqrt();
			let l_pan = (1.0 - pan_value).sqrt();

			let attenuation = attenuation_curve.next().clamp(0.0, 1.0);

			*out_l = filtered_sample * l_pan * attenuation;
			*out_r = filtered_sample * r_pan * attenuation;
		}
	}
}






pub struct EmitterNode {
	// parameter
	freq_parameter: FloatParameter,

	// state
	osc_0: SquareWave,
	osc_1: SquareWave,
	lfo: SineWave,
}


impl EmitterNode {
	fn new(freq_parameter: FloatParameter) -> EmitterNode {
		EmitterNode {
			freq_parameter,
			osc_0: SquareWave::new(),
			osc_1: SquareWave::new(),
			lfo: SineWave::new(),
		}
	}
}

impl Node for EmitterNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { false }

	fn process(&mut self, ProcessContext{eval_ctx, inputs, output}: ProcessContext<'_>) {
		assert!(inputs.is_empty());

		let mut freq_curve = self.freq_parameter.read(output.len().min(100));
		let sample_dt = 1.0 / eval_ctx.sample_rate;

		for out_sample in output.iter_mut() {
			let freq = freq_curve.next();

			let lfo = self.lfo.next(freq/100.0, sample_dt) * 0.4 + 0.6;
			let sample = self.osc_0.next(freq, sample_dt)
				+ self.osc_1.next(freq*4.0, sample_dt);

			*out_sample = sample*lfo*0.5;
		}
	}
}



struct SineWave {
	phase: f32,
}

impl SineWave {
	fn new() -> SineWave {
		SineWave { phase: 0.0 }
	}

	fn next(&mut self, freq: f32, sample_dt: f32) -> f32 {
		let sample = (self.phase * TAU).sin();
		self.phase = (self.phase + freq * sample_dt).fract();
		sample
	}
}


struct SquareWave {
	phase: f32,
}

impl SquareWave {
	fn new() -> SquareWave {
		SquareWave { phase: 0.0 }
	}

	fn next(&mut self, freq: f32, sample_dt: f32) -> f32 {
		let prev_phase = self.phase;
		self.phase = (self.phase + freq * sample_dt).fract();
		1.0 - (prev_phase + 0.5).floor() * 2.0
	}
}



struct LowPass {
	filtered_sample: f32,
}

impl LowPass {
	fn new() -> LowPass {
		LowPass { filtered_sample: 0.0 }
	}

	fn next(&mut self, in_sample: f32, cutoff: f32, sample_dt: f32) -> f32 {
		let a = sample_dt / (sample_dt + 1.0 / (TAU * cutoff));
		self.filtered_sample = a.lerp(self.filtered_sample, in_sample);
		self.filtered_sample
	}
}





struct DelayLineNode {
	feedback_parameter: FloatParameter,

	data: Vec<f32>,
	position: usize,
}

impl DelayLineNode {
	fn new(size: usize, feedback_parameter: FloatParameter) -> DelayLineNode {
		DelayLineNode {
			feedback_parameter,

			data: vec![0.0; size],
			position: 0,
		}
	}
}

impl Node for DelayLineNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { false }

	fn process(&mut self, ProcessContext{inputs, output, eval_ctx}: ProcessContext<'_>) {
		assert!(inputs.len() == 1);
		assert!(!output.stereo());

		let input = &inputs[0];
		assert!(!input.stereo());

		let mut feedback_curve = self.feedback_parameter.read(output.len());

		for (out_sample, &in_sample) in iter::zip(output.iter_mut(), input.iter()) {
			*out_sample = in_sample - self.data[self.position] * feedback_curve.next();
			self.data[self.position] = *out_sample;
			self.position = (self.position+1) % self.data.len();
		}
	}
}


