use toybox::prelude::*;
use toybox::audio::{*, nodes::*};

pub struct AudioTestController {
	plink_sound_key: toybox::audio::SoundId,
	plink_mixer_node: toybox::audio::NodeId,

	global_volume_send: ParameterChannelSender<f32>,
	global_volume: f32,

	drone_volume_send: ParameterChannelSender<f32>,
	drone_volume: f32,

	freq_send: ParameterChannelSender<f32>,
	freq: f32,

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

		let (global_volume_send, global_volume_recv) = parameter_channel(1.0);
		let global_mixer = engine.audio.add_node_with_send(MixerNode::new_stereo(global_volume_recv), engine.audio.output_node());


		let plink_sound_key = engine.audio.add_sound(buffer);
		let plink_mixer_node = engine.audio.add_node_with_send(MixerNode::new(0.5), global_mixer);

		
		let mod_mixer_node = engine.audio.add_node_with_send(MixerNode::new_stereo(1.0), global_mixer);
		let (freq_send, freq_recv) = parameter_channel(10.0);

		engine.audio.add_node_with_send(OscillatorNode::new(freq_recv), mod_mixer_node);



		let (drone_volume_send, drone_volume_recv) = parameter_channel(0.00);
		let drone_mixer_node = engine.audio.add_node_with_send(MixerNode::new_stereo(drone_volume_recv), global_mixer);

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
			plink_mixer_node,

			global_volume_send,
			global_volume: 100.0,

			drone_volume_send,
			drone_volume: 3.0,

			freq_send,
			freq: 440.0,

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

			if imgui::Slider::new("global volume", 0.0, 100.0).display_format("%.2f%%").build(ui, &mut self.global_volume) {
				self.global_volume_send.send(volume_to_gain(self.global_volume / 100.0));
			}

			if imgui::Slider::new("drone volume", 0.0, 100.0).display_format("%.2f%%").build(ui, &mut self.drone_volume) {
				self.drone_volume_send.send(volume_to_gain(self.drone_volume / 100.0));
			}

			if imgui::Slider::new("freq", 10.0, 880.0).build(ui, &mut self.freq) {
				self.freq_send.send(self.freq);
			}
		}
	}
}


fn volume_to_gain(volume: f32) -> f32 {
	volume.powi(3)
}