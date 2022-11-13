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
		}
	}

	pub fn update(&mut self, audio: &mut audio::AudioSystem, ui: &imgui::Ui<'static>) {
		let bps = self.bpm/60.0;
		self.time += 4.0 * bps/60.0; // quarter notes

		let cursor = self.time.trunc() as usize % self.sequence.len();
		if self.sequence_cursor != cursor {
			let frequency = self.sequence[cursor].to_frequency();
			let mixer_id = self.mixer_id;

			audio.queue_update(move |graph| {
				let envelope = env::AR::new(0.02, 0.5).exp(4.0);
				let node = gen::GeneratorNode::new_triangle(frequency)
					.envelope(envelope)
					.build();
				graph.add_node(node, mixer_id);
			});

			self.sequence_cursor = cursor;
		}

		imgui::Slider::new("BPM", 40.0, 240.0)
			.build(ui, &mut self.bpm);

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


	// let mut pc_index = pitch_classes.iter().position(|&pc| pc == *pitch_class).unwrap();
	// if ui.combo("Pitch Class", &mut pc_index, &pitch_classes, |v| format!("{v:?}").into()) {
	// 	*pitch_class = pitch_classes[pc_index];
	// 	true
	// } else {
	// 	false
	// }

	let mut changed = false;

	for pc in pitch_classes {
		let _style_token = (pc == *pitch_class)
			.then(|| {
				ui.push_style_color(imgui::StyleColor::FrameBg, [1.0, 1.0, 0.4, 1.0])
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