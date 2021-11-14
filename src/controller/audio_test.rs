use toybox::prelude::*;
use toybox::audio::nodes::*;

pub struct AudioTestController {
	plink_sound_key: toybox::audio::SoundId,
	mixer_node: toybox::audio::NodeId,

	pub clicked: bool,
}

impl AudioTestController {
	pub fn new(engine: &mut toybox::Engine) -> AudioTestController {
		let mut buffer = vec![0.0; 44100/4];
		for (index, sample) in buffer.iter_mut().enumerate() {
			let t = index as f32 / 44100.0;
			// *sample = ((1.0 + t).powi(2) * 220.0).sin() * (1.0 - t).max(0.0).powi(2);

			*sample = (880.0 * TAU * t).sin() * (1.0 - t*8.0).max(0.0).powi(4);
		}

		let plink_sound_key = engine.audio.add_sound(buffer);
		let mixer_node = engine.audio.add_node_with_send(MixerNode::new(0.5), engine.audio.output_node());

		

		let drone_mixer_node = engine.audio.add_node_with_send(MixerNode::new_stereo(0.03), engine.audio.output_node());

		let panner_node = engine.audio.add_node_with_send(PannerNode::new(1.0), drone_mixer_node);
		let mixer_node = engine.audio.add_node_with_send(MixerNode::new(2.0), panner_node);
		for freq in [55.0, 330.0] {
			engine.audio.add_node_with_send(OscillatorNode::new(freq), mixer_node);
		}

		let panner_node = engine.audio.add_node_with_send(PannerNode::new(-1.0), drone_mixer_node);
		let mixer_node = engine.audio.add_node_with_send(MixerNode::new(1.0), panner_node);
		for freq in [220.0, 110.0, 550.0] {
			engine.audio.add_node_with_send(OscillatorNode::new(freq), mixer_node);
		}


		AudioTestController {
			plink_sound_key,
			mixer_node,

			clicked: false,
		}
	}

	pub fn update(&mut self, engine: &mut toybox::Engine) {
		let ui = engine.imgui.frame();

		self.clicked = false;

		if let Some(_window) = imgui::Window::new("Audio").begin(ui) {
			if ui.button("Play") {
				let node = SamplerNode::new(self.plink_sound_key);
				engine.audio.add_node_with_send(node, self.mixer_node);

				self.clicked = true;
			}
		}
	}
}