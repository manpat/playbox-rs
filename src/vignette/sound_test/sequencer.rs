use toybox::prelude::*;
use audio::node_builder::*;
use audio::generator as gen;
use audio::envelope as env;


pub struct SequencerPanel {
	mixer_id: audio::NodeId,

	time: f32,
}

impl SequencerPanel {
	pub fn new(mixer_id: audio::NodeId) -> SequencerPanel {
		SequencerPanel {
			mixer_id,
			time: 0.0,
		}
	}

	pub fn update(&mut self, audio: &mut audio::AudioSystem, ui: &imgui::Ui<'static>) {
		self.time += 1.0/60.0;

		ui.text("Foo");
	}
}