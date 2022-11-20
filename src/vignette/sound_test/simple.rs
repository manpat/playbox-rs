use toybox::prelude::*;
use audio::node_builder::*;
use audio::generator as gen;
use audio::envelope as env;

use audio::Envelope;


pub struct SimplePanel {
	global_state: super::GlobalAudioState,

	pulse_width: f32,
	base_frequency: f32,

	env_attack: f32,
	env_release: f32,
	env_exp_attack: f32,
	env_exp_release: f32,

	envelope_shape: Vec<f32>,
}

impl SimplePanel {
	pub fn new(global_state: super::GlobalAudioState) -> SimplePanel {
		let mut simple = SimplePanel {
			global_state,

			pulse_width: 0.25,
			base_frequency: 220.0,

			env_attack: 0.03,
			env_release: 1.5,

			env_exp_attack: 1.0 / 4.0,
			env_exp_release: 4.0,

			envelope_shape: Vec::new(),
		};

		simple.rebuild_envelope();

		simple
	}

	pub fn update(&mut self, audio: &mut audio::AudioSystem, ui: &imgui::Ui<'static>) {
		if let Some(_table) = ui.begin_table_with_flags("Simple Table", 2, imgui::TableFlags::BORDERS_INNER | imgui::TableFlags::RESIZABLE) {
			ui.table_next_column();
			self.draw_play_buttons(audio, ui);

			ui.table_next_column();
			self.draw_controls(ui);

			ui.table_next_column();

			let midi_note_f = audio::util::frequency_to_midi_note(self.base_frequency);
			let midi_note = midi_note_f.trunc() as i32;
			let cents = (midi_note_f.fract() * 100.0) as i32;

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

			if cents != 0 {
				ui.label_text("Note", format!("{note_name}{octave} +{cents}cents"));
			} else {
				ui.label_text("Note", format!("{note_name}{octave}"));
			}


			ui.table_next_column();

			ui.plot_lines("Envelope", &self.envelope_shape)
				.scale_min(0.0)
				.scale_max(1.0)
				.graph_size([0.0, 100.0])
				.build();
		}
	}

	fn rebuild_envelope(&mut self) {
		let mut envelope = env::AR::new(self.env_attack, self.env_release)
			.exp2(self.env_exp_attack, self.env_exp_release);

		let total_time = self.env_attack + self.env_release;
		let resolution = total_time / 1024.0;

		self.envelope_shape.clear();
		while !envelope.is_finished() {
			self.envelope_shape.push(envelope.next(resolution));
		}

		for _ in 0..10 {
			self.envelope_shape.push(0.0);
		}
	}

	fn draw_play_buttons(&mut self, audio: &mut audio::AudioSystem, ui: &imgui::Ui<'static>) {
		let SimplePanel {
			ref global_state,
			base_frequency,
			pulse_width,
			env_attack,
			env_release,
			env_exp_attack,
			env_exp_release,
			..
		} = *self;

		let mixer_id = global_state.mixer_id;

		let envelope = env::AR::new(env_attack, env_release)
			.exp2(env_exp_attack, env_exp_release);

		if ui.button("Play") {
			audio.queue_update(move |graph| {
				let noise = gen::Noise::new().envelope(env::AR::new(0.01, 0.2).exp(4.0));
				let osc = gen::GeneratorNode::new_sine(base_frequency).envelope(env::AR::new(0.03, 0.5).exp(4.0));
				let node = (noise, osc).add().low_pass(200.0).build();
				graph.add_node(node, mixer_id);
			});
		}

		ui.same_line();

		if ui.button("Play 2") {
			audio.queue_update(move |graph| {
				let noise = gen::Noise::new().envelope(env::AR::new(0.3, 1.5).exp(4.0));
				let osc1 = gen::GeneratorNode::new_triangle(base_frequency / 2.0).envelope(env::AR::new(0.2, 0.5).exp(4.0));
				let osc2 = gen::GeneratorNode::new_pulse(base_frequency, 0.1).envelope(env::AR::new(0.03, 2.0).exp(4.0));
				let node = (noise, osc1, osc2).add().low_pass(200.0).high_pass(2.0).build();
				graph.add_node(node, mixer_id);
			});
		}

		ui.same_line();

		if ui.button("Play 3") {
			audio.queue_update(move |graph| {
				let lfo_rate = env::Ramp::new((env_attack + env_release)*0.7, 16.0, 2.0).to_parameter();

				let parts = (
					gen::GeneratorNode::new_pulse(base_frequency*3.0, 0.3),
					gen::GeneratorNode::new_square(base_frequency*2.0),
					(gen::GeneratorNode::new_sine(base_frequency + 2.0), gen::GeneratorNode::new_triangle(base_frequency/2.0)).add(),
					gen::GeneratorNode::new_triangle(lfo_rate).gain_bias(0.3, 0.7), // lfo
				);

				let node = parts.multiply()
					.envelope(envelope)
					.high_pass(10.0)
					.build();
				graph.add_node(node, mixer_id);
			});
		}

		if ui.button("Sine") {
			audio.queue_update(move |graph| {
				let node = gen::GeneratorNode::new_sine(base_frequency)
					.envelope(envelope)
					.build();
				graph.add_node(node, mixer_id);
			});
		}

		if ui.button("Triangle") {
			audio.queue_update(move |graph| {
				let node = gen::GeneratorNode::new_triangle(base_frequency)
					.envelope(envelope)
					.build();
				graph.add_node(node, mixer_id);
			});
		}

		if ui.button("Square") {
			audio.queue_update(move |graph| {
				let node = gen::GeneratorNode::new_square(base_frequency)
					.envelope(envelope)
					.build();
				graph.add_node(node, mixer_id);
			});
		}

		if ui.button("Saw") {
			audio.queue_update(move |graph| {
				let node = gen::GeneratorNode::new_saw(base_frequency)
					.envelope(envelope)
					.build();
				graph.add_node(node, mixer_id);
			});
		}

		if ui.button("Pulse") {
			audio.queue_update(move |graph| {
				let node = gen::GeneratorNode::new(base_frequency, move |p| gen::pulse_wave(p, pulse_width))
					.envelope(envelope)
					.high_pass(1.0)
					.build();
				graph.add_node(node, mixer_id);
			});
		}

		ui.same_line();

		imgui::Slider::new("Width", 0.0, 1.0)
			.build(ui, &mut self.pulse_width);
	}


	fn draw_controls(&mut self, ui: &imgui::Ui<'static>) {
		let midi_note_f = audio::util::frequency_to_midi_note(self.base_frequency);
		let mut midi_note = midi_note_f.trunc() as i32;
		let mut cents = (midi_note_f.fract() * 100.0) as i32;

		imgui::Slider::new("Frequency", 22.0, 880.0)
			.flags(imgui::SliderFlags::LOGARITHMIC)
			.build(ui, &mut self.base_frequency);


		if imgui::Slider::new("Midi Note", 16, 81)
			.build(ui, &mut midi_note)
		{
			let real_note = midi_note as f32 + cents as f32/100.0;
			self.base_frequency = audio::util::midi_note_to_frequency(real_note);
		}

		if imgui::Slider::new("Cents", 0, 99)
			.build(ui, &mut cents)
		{
			let real_note = midi_note as f32 + cents as f32/100.0;
			self.base_frequency = audio::util::midi_note_to_frequency(real_note);
		}

		if ui.button("Sync To Oscilloscope") {
			self.base_frequency = 44100.0 / 256.0;
		}

		ui.same_line();
		if ui.button("+ Octave") {
			self.base_frequency *= 2.0;
		}

		ui.same_line();
		if ui.button("- Octave") {
			self.base_frequency /= 2.0;
		}

		ui.new_line();

		let mut envelope_changed = false;

		envelope_changed |= imgui::Slider::new("Attack", 0.01, 4.0)
			.build(ui, &mut self.env_attack);

		envelope_changed |= imgui::Slider::new("Attack Curve", 0.01, 4.0)
			.flags(imgui::SliderFlags::LOGARITHMIC)
			.build(ui, &mut self.env_exp_attack);

		envelope_changed |= imgui::Slider::new("Release", 0.01, 4.0)
			.build(ui, &mut self.env_release);

		envelope_changed |= imgui::Slider::new("Release Curve", 0.01, 4.0)
			.flags(imgui::SliderFlags::LOGARITHMIC)
			.build(ui, &mut self.env_exp_release);

		if envelope_changed {
			self.rebuild_envelope();
		}
	}
}