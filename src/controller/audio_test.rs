use toybox::prelude::*;
use toybox::audio::nodes::*;

pub struct AudioTestController {
	plink_sound_key: audio::SoundId,
	plink_mixer_node: audio::NodeId,

	pub clicked: bool,
}

impl AudioTestController {
	pub fn new(engine: &mut toybox::Engine) -> AudioTestController {
		// let buffer = (0..44100/4)
		// 	.map(|index| {
		// 		let t = index as f32 / 44100.0;
		// 		(880.0 * TAU * t).sin() * (1.0 - t*8.0).max(0.0).powi(4)
		// 	})
		// 	.collect();

		let buffer = super::load_audio_buffer("assets/chime.ogg").unwrap();

		let plink_sound_key = engine.audio.add_sound(buffer);
		let plink_mixer_node = engine.audio.add_node_with_send(MixerNode::new(0.1), engine.audio.output_node());


		let drone_mixer_node = engine.audio.add_node_with_send(MixerNode::new_stereo(0.001), engine.audio.output_node());

		engine.audio.update_graph(move |graph| {
			let panner_node = graph.add_node(PannerNode::new(1.0), true);
			let mixer_node = graph.add_node(MixerNode::new(2.0), true);

			for freq in [55.0, 330.0] {
				let node = graph.add_node(OscillatorNode::new(freq), false);
				graph.add_send(node, mixer_node);
			}

			graph.add_send_chain(&[mixer_node, panner_node, drone_mixer_node]);
		});

		engine.audio.update_graph(move |graph| {
			let panner_node = graph.add_node(PannerNode::new(-1.0), true);
			let mixer_node = graph.add_node(MixerNode::new(1.0), true);

			for freq in [220.0, 110.0, 550.0] {
				let node = graph.add_node(OscillatorNode::new(freq), false);
				graph.add_send(node, mixer_node);
			}

			graph.add_send_chain(&[mixer_node, panner_node, drone_mixer_node]);
		});


		AudioTestController {
			plink_sound_key,
			plink_mixer_node,

			clicked: false,
		}
	}

	pub fn update(&mut self, engine: &mut toybox::Engine) {
		let ui = engine.imgui.frame();

		self.clicked = false;

		if let Some(_window) = imgui::Window::new("Audio").begin(ui) {
			if ui.button("Play") {
				let node = SamplerNode::new(self.plink_sound_key);
				engine.audio.add_node_with_send(node, self.plink_mixer_node);

				self.clicked = true;
			}
		}
	}
}

