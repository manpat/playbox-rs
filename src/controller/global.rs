use toybox::prelude::*;
use toybox::input::raw::Scancode;
use toybox::audio::{self, SoundAssetID};


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

	pluck_sound_id: SoundAssetID,
	stereo_sound_id: SoundAssetID,
	static_ogg_sound_id: SoundAssetID,
	file_ogg_sound_id: SoundAssetID,

	should_quit: bool,
	wireframe_enabled: bool,
}

impl GlobalController {
	pub fn new(engine: &mut toybox::Engine) -> Result<GlobalController, Box<dyn Error>> {
		let actions = GlobalActions::new(&mut engine.input);
		engine.input.enter_context(actions.context_id());

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

			let buffer = audio::Buffer::from_mono_samples(samples);
			engine.audio.register_buffer(buffer)
		};

		let stereo_sound_id = {
			let framerate = 44100;
			let freq = 660.0;

			let attack_t = framerate as f32 * 0.01;
			let release_t = framerate as f32 * 4.0;

			let sound_t = attack_t + release_t;
			let buffer_size = sound_t as usize;

			let samples = (0..buffer_size)
				.map(move |x| {
					let x = x as f32;
					let attack = (x / attack_t).min(1.0);
					let release = (1.0 - (x - attack_t) / (sound_t - attack_t)).powf(10.0);

					let envelope = attack*release;

					(x * freq / framerate as f32 * PI).sin() * envelope
				})
				.flat_map(|sample| [sample, -sample]);

			let buffer = audio::Buffer::from_stereo_samples(samples);
			engine.audio.register_buffer(buffer)
		};

		let static_ogg_sound_id = {
			let raw_data = include_bytes!("../../assets/forest.ogg");
			let stream = audio::FileStream::from_vorbis_static(raw_data)?;
			engine.audio.register_file_stream(stream)
		};

		let file_ogg_sound_id = {
			let stream = audio::FileStream::from_vorbis_file("assets/forest.ogg")?;
			engine.audio.register_file_stream(stream)
		};


		Ok(GlobalController {
			actions,

			pluck_sound_id,
			stereo_sound_id,
			static_ogg_sound_id,
			file_ogg_sound_id,

			should_quit: false,
			wireframe_enabled: false,
		})
	}

	pub fn update(&mut self, engine: &mut toybox::Engine) {
		if engine.input.frame_state().active(self.actions.quit) {
			self.should_quit = true
		}

		if engine.input.frame_state().active(self.actions.toggle_wireframe) {
			self.wireframe_enabled = !self.wireframe_enabled;
			engine.gfx.set_wireframe(self.wireframe_enabled);
		}

		if engine.input.frame_state().active(self.actions.play_sound) {
			engine.audio.play_one_shot(self.pluck_sound_id);
		}

		if engine.input.frame_state().active(self.actions.play_stereo_sound) {
			engine.audio.play_one_shot(self.stereo_sound_id);
		}

		if engine.input.frame_state().active(self.actions.play_static_stream_sound) {
			engine.audio.play_one_shot(self.static_ogg_sound_id);
		}

		if engine.input.frame_state().active(self.actions.play_file_stream_sound) {
			engine.audio.play_one_shot(self.file_ogg_sound_id);
		}

	}

	pub fn should_quit(&self) -> bool {
		self.should_quit
	}
}
