use toybox::prelude::*;
use audio::node_builder::*;
use audio::generator as gen;
use audio::envelope as env;


pub struct SequencerPanel {
	mixer_id: audio::NodeId,
	sequence: Vec<audio::util::Pitch>,
	sequence_cursor: usize,

	time: f32,
	bpm: f32,
	gain: f32,

	lpf: f32,
	hpf: f32,
	q: f32,

	selected_waveform: usize,
}

impl SequencerPanel {
	pub fn new(mixer_id: audio::NodeId) -> SequencerPanel {
		let fundamental = 45.0;

		let sequence = [
			fundamental,
			fundamental + 5.0,
			fundamental + 8.0,
			fundamental + 12.0,
		];

		SequencerPanel {
			mixer_id,
			sequence: sequence.into_iter().map(audio::util::Pitch::from_midi).collect(),
			sequence_cursor: usize::MAX,
			time: 0.0,

			bpm: 80.0,
			gain: 1.0,

			lpf: 16000.0,
			hpf: 1.0,
			q: 0.0,

			selected_waveform: 0,
		}
	}

	pub fn update(&mut self, audio: &mut audio::AudioSystem, ui: &imgui::Ui<'static>) {
		let bps = self.bpm/60.0;
		self.time += 4.0 * bps/60.0; // quarter notes

		let cursor = self.time.trunc() as usize % self.sequence.len();
		if self.sequence_cursor != cursor {
			let frequency = self.sequence[cursor].to_frequency();
			let gain = self.gain;
			let lpf = self.lpf;
			let hpf = self.hpf;
			let q = self.q;
			let mixer_id = self.mixer_id;
			let selected_waveform = self.selected_waveform;

			audio.queue_update(move |graph| {
				let envelope = env::AR::new(0.02, 0.5).exp(4.0);
				match selected_waveform {
					0 => {
						let node = gen::GeneratorNode::new_sine(frequency)
							.envelope(envelope)
							.gain(gain)
							.low_pass(lpf)
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					1 => {
						let node = gen::GeneratorNode::new_triangle(frequency)
							.envelope(envelope)
							.gain(gain)
							.low_pass(lpf)
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					2 => {
						let node = gen::GeneratorNode::new_square(frequency)
							.envelope(envelope)
							.gain(gain)
							.low_pass(lpf)
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					3 => {
						let node = gen::GeneratorNode::new_saw(frequency)
							.envelope(envelope)
							.gain(gain)
							.low_pass(lpf)
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					4 => {
						let oscs = (gen::GeneratorNode::new_saw(frequency), gen::GeneratorNode::new_saw(frequency*1.01)).add();

						let node = oscs
							.envelope(envelope)
							.gain(gain)
							.low_pass(lpf)
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					5 => {
						let osc = gen::GeneratorNode::new_square(frequency);
						let f = 2.0 * (PI * lpf / 44100.0).sin();
						let fb = q + q / (1.0 - f);
						let mut buf0 = 0.0;
						let mut buf1 = 0.0;

						let node = osc
							.envelope(envelope)
							.gain(gain)
							.effect(move |sample: f32| {
								let hp = sample - buf0;
								let bp = buf0 - buf1;
								buf0 = buf0 + f * (hp + fb * bp);
								buf1 = buf1 + f * (buf0 - buf1);
								buf1
							})
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					6 => {
						let osc = gen::Noise::new();
						let f = 2.0 * (PI * frequency / 44100.0).sin();
						let fb = q + q / (1.0 - f);
						let mut buf0 = 0.0;
						let mut buf1 = 0.0;

						let node = osc
							.envelope(envelope)
							.gain(gain)
							.effect(move |sample: f32| {
								let hp = sample - buf0;
								let bp = buf0 - buf1;
								buf0 = buf0 + f * (hp + fb * bp);
								buf1 = buf1 + f * (buf0 - buf1);
								buf1
							})
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					7 => {
						let modulator = gen::GeneratorNode::new_sine(6.0)
							.effect(move |sample: f32| frequency + 10.0 * (sample * 0.5 + 0.5))
							.to_parameter();

						let node = gen::GeneratorNode::new_triangle(modulator)
							.envelope(envelope)
							.gain(gain)
							.low_pass(lpf)
							.high_pass(hpf)
							.build();
						graph.add_node(node, mixer_id);
					}

					_ => {}
				}
			});

			self.sequence_cursor = cursor;
		}

		imgui::Slider::new("BPM", 40.0, 240.0)
			.build(ui, &mut self.bpm);

		imgui::Slider::new("Gain", 0.0, 32.0)
			.flags(imgui::SliderFlags::LOGARITHMIC)
			.build(ui, &mut self.gain);

		ui.combo_simple_string("Waveform", &mut self.selected_waveform, &[
			"Sine", "Triangle", "Square", "Saw", "DoubleSaw", "FilterTest", "FilteredNoise", "ModulatedTri"]);

		imgui::Slider::new("LPF", 1.0, 16000.0)
			.flags(imgui::SliderFlags::LOGARITHMIC)
			.build(ui, &mut self.lpf);

		imgui::Slider::new("HPF", 1.0, 16000.0)
			.flags(imgui::SliderFlags::LOGARITHMIC)
			.build(ui, &mut self.hpf);

		let mut transformed_q = 1.0 - (1.0 - self.q).powf(1.0 / 2.0);
		imgui::Slider::new("Q", 0.0, 1.0)
			.build(ui, &mut transformed_q);

		self.q = 1.0 - (1.0 - transformed_q).powf(2.0);
		ui.text(format!("real Q: {:.3}", self.q));


		if let Some(_table) = ui.begin_table_with_flags("sequence", self.sequence.len(), imgui::TableFlags::BORDERS_INNER) {
			for (idx, item) in self.sequence.iter_mut().enumerate() {
				ui.table_next_column();

				let _id_token = ui.push_id(idx as i32);

				{
					let _style_token = (idx == cursor)
						.then(|| {
							ui.push_style_color(imgui::StyleColor::Text, [1.0, 1.0, 0.4, 1.0])
						});


					let audio::util::Pitch{pitch_class, octave, cents} = *item;
					ui.text(format!("{pitch_class}{octave} +{cents}cents"));
				}

				pitch_class_selector(ui, &mut item.pitch_class);

				imgui::Slider::new("Octave", 0, 8)
					.build(ui, &mut item.octave);
			}
		}
	}
}


fn pitch_class_selector(ui: &imgui::Ui<'_>, pitch_class: &mut audio::util::PitchClass) -> bool {
	let pitch_classes = [
		audio::util::PitchClass::C,
		audio::util::PitchClass::Cs,
		audio::util::PitchClass::D,
		audio::util::PitchClass::Ds,
		audio::util::PitchClass::E,
		audio::util::PitchClass::F,
		audio::util::PitchClass::Fs,
		audio::util::PitchClass::G,
		audio::util::PitchClass::Gs,
		audio::util::PitchClass::A,
		audio::util::PitchClass::As,
		audio::util::PitchClass::B,
	];

	let mut changed = false;

	for pc in pitch_classes {
		let _style_token = (pc == *pitch_class)
			.then(|| {
				ui.push_style_color(imgui::StyleColor::Button, [0.5, 0.5, 0.2, 1.0])
			});

		if ui.button(format!("{pc}")) {
			*pitch_class = pc;
			changed = true;
		}

		ui.same_line();
	}

	ui.new_line();
	changed
}