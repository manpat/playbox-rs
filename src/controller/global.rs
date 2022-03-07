use toybox::prelude::*;
use toybox::input::raw::Scancode;

use audio::nodes::{MixerNode, SamplerNode};


toybox::declare_input_context! {
	struct GlobalActions "Global" {
		trigger quit { "Quit" [Scancode::Escape] }
		trigger toggle_wireframe { "Toggle Wireframe" [Scancode::Z] }
		trigger play_sound { "Play Sound" [Scancode::Num1] }
		trigger play_stereo_sound { "Play Stereo Sound" [Scancode::Num2] }
		trigger play_static_stream_sound { "Play Static Streamed Sound" [Scancode::Num3] }
		trigger play_file_stream_sound { "Play File Streamed Sound" [Scancode::Num4] }
	}
}


pub struct GlobalController {
	actions: GlobalActions,

	pluck_sound_id: audio::SoundId,
	// stereo_sound_id: audio::SoundId,
	// static_ogg_sound_id: audio::SoundId,
	// file_ogg_sound_id: audio::SoundId,

	soundbus: audio::NodeId,

	should_quit: bool,
	wireframe_enabled: bool,
}

impl GlobalController {
	pub fn new(engine: &mut toybox::Engine) -> Result<GlobalController, Box<dyn Error>> {
		let pluck_sound_id = {
			let framerate = 44100;
			let freq = 440.0;

			let attack_t = framerate as f32 * 0.01;
			let release_t = framerate as f32 * 0.2;

			let sound_t = attack_t + release_t;
			let buffer_size = sound_t as usize;

			let samples = (0..buffer_size)
				.map(move |x| {
					let x = x as f32;
					let attack = (x / attack_t).min(1.0);
					let release = (1.0 - (x - attack_t) / (sound_t - attack_t)).powf(10.0);

					let envelope = attack*release;

					(x * freq / framerate as f32 * PI).sin() * envelope
				});

			// let buffer = audio::Buffer::from_mono_samples(samples);
			// engine.audio.register_buffer(buffer)
			engine.audio.add_sound(samples.collect())
		};

		// let stereo_sound_id = {
		// 	let framerate = 44100;
		// 	let freq = 660.0;

		// 	let attack_t = framerate as f32 * 0.01;
		// 	let release_t = framerate as f32 * 4.0;

		// 	let sound_t = attack_t + release_t;
		// 	let buffer_size = sound_t as usize;

		// 	let samples = (0..buffer_size)
		// 		.map(move |x| {
		// 			let x = x as f32;
		// 			let attack = (x / attack_t).min(1.0);
		// 			let release = (1.0 - (x - attack_t) / (sound_t - attack_t)).powf(10.0);

		// 			let envelope = attack*release;

		// 			(x * freq / framerate as f32 * PI).sin() * envelope
		// 		})
		// 		.flat_map(|sample| [sample, -sample]);

		// 	let buffer = audio::Buffer::from_stereo_samples(samples);
		// 	engine.audio.register_buffer(buffer)
		// };

		// let static_ogg_sound_id = {
		// 	let raw_data = include_bytes!("../../assets/forest.ogg");
		// 	let stream = audio::FileStream::from_vorbis_static(raw_data)?;
		// 	engine.audio.register_file_stream(stream)
		// };

		// let file_ogg_sound_id = {
		// 	let sound = super::load_audio_buffer("assets/forest.ogg")?;
		// 	engine.audio.add_sound(sound)
		// };


		// let soundbus_bottom = engine.audio.new_bus("Global Bottom");
		// let soundbus_top = engine.audio.new_bus("Global Top");

		// engine.audio.get_bus_mut(soundbus_top).unwrap()
		// 	.set_gain(0.1);

		// engine.audio.get_bus_mut(soundbus_bottom).unwrap()
		// 	.set_send_bus(soundbus_top);

		let soundbus = engine.audio.add_node_with_send(MixerNode::new(0.1), engine.audio.output_node());

		Ok(GlobalController {
			actions: GlobalActions::new_active(&mut engine.input),

			pluck_sound_id,
			// stereo_sound_id,
			// static_ogg_sound_id,
			// file_ogg_sound_id,

			soundbus,

			should_quit: false,
			wireframe_enabled: false,
		})
	}

	pub fn update(&mut self, engine: &mut toybox::Engine) {
		let input_state = engine.input.frame_state();
		// let bus = engine.audio.get_bus_mut(self.soundbus).unwrap();

		if input_state.active(self.actions.quit) {
			self.should_quit = true
		}

		if input_state.active(self.actions.toggle_wireframe) {
			self.wireframe_enabled = !self.wireframe_enabled;
			engine.gfx.render_state().set_wireframe(self.wireframe_enabled);
		}

		if input_state.active(self.actions.play_sound) {
			engine.audio.update_graph(|graph| {
				let node_id = graph.add_node(SamplerNode::new(self.pluck_sound_id), false);
				graph.add_send(node_id, self.soundbus);
			});
		}

		// if input_state.active(self.actions.play_stereo_sound) {
		// 	bus.start_sound(self.stereo_sound_id);
		// }

		// if input_state.active(self.actions.play_static_stream_sound) {
		// 	bus.start_sound(self.static_ogg_sound_id);
		// }

		// if input_state.active(self.actions.play_file_stream_sound) {
		// 	engine.audio.update_graph(|graph| {
		// 		let node_id = graph.add_node(SamplerNode::new(self.file_ogg_sound_id), false);
		// 		graph.add_send(node_id, self.soundbus);
		// 	});
		// }
	}

	pub fn should_quit(&self) -> bool {
		self.should_quit
	}
}
