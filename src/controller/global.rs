use toybox::prelude::*;
use toybox::input::{ContextID, ActionID};
use toybox::input::raw::{Scancode, MouseButton};
use toybox::audio::{self, SoundAssetID};


pub struct GlobalController {
	quit_action: ActionID,
	toggle_wireframe_action: ActionID,
	play_sound_action: ActionID,
	play_stereo_sound_action: ActionID,
	play_static_stream_sound_action: ActionID,
	play_file_stream_sound_action: ActionID,

	pluck_sound_id: SoundAssetID,
	stereo_sound_id: SoundAssetID,
	static_ogg_sound_id: SoundAssetID,
	file_ogg_sound_id: SoundAssetID,

	should_quit: bool,
	wireframe_enabled: bool,
}

impl GlobalController {
	pub fn new(engine: &mut toybox::Engine) -> Result<GlobalController, Box<dyn Error>> {
		let mut global_input_ctx = engine.input.new_context("Global");
		let quit_action = global_input_ctx.new_trigger("Quit", Scancode::Escape);
		let toggle_wireframe_action = global_input_ctx.new_trigger("Toggle Wireframe", Scancode::Z);
		let play_sound_action = global_input_ctx.new_trigger("Play Sound", Scancode::Num1);
		let play_stereo_sound_action = global_input_ctx.new_trigger("Play Stereo Sound", Scancode::Num2);
		let play_static_stream_sound_action = global_input_ctx.new_trigger("Play Static Streamed Sound", Scancode::Num3);
		let play_file_stream_sound_action = global_input_ctx.new_trigger("Play File Streamed Sound", Scancode::Num4);
		let global_input_ctx = global_input_ctx.build();

		engine.input.enter_context(global_input_ctx);

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
			let stream = audio::Stream::from_vorbis_static(raw_data)?;
			engine.audio.register_stream(stream)
		};

		let file_ogg_sound_id = {
			let stream = audio::Stream::from_vorbis_file("assets/forest.ogg")?;
			engine.audio.register_stream(stream)
		};


		Ok(GlobalController {
			quit_action,
			toggle_wireframe_action,
			play_sound_action,
			play_stereo_sound_action,
			play_static_stream_sound_action,
			play_file_stream_sound_action,

			pluck_sound_id,
			stereo_sound_id,
			static_ogg_sound_id,
			file_ogg_sound_id,

			should_quit: false,
			wireframe_enabled: false,
		})
	}

	pub fn update(&mut self, engine: &mut toybox::Engine) {
		if engine.input.frame_state().active(self.quit_action) {
			self.should_quit = true
		}

		if engine.input.frame_state().active(self.toggle_wireframe_action) {
			self.wireframe_enabled = !self.wireframe_enabled;
			engine.gl_ctx.set_wireframe(self.wireframe_enabled);
		}

		if engine.input.frame_state().active(self.play_sound_action) {
			engine.audio.play_one_shot(self.pluck_sound_id);
		}

		if engine.input.frame_state().active(self.play_stereo_sound_action) {
			engine.audio.play_one_shot(self.stereo_sound_id);
		}

		if engine.input.frame_state().active(self.play_static_stream_sound_action) {
			engine.audio.play_one_shot(self.static_ogg_sound_id);
		}

		if engine.input.frame_state().active(self.play_file_stream_sound_action) {
			engine.audio.play_one_shot(self.file_ogg_sound_id);
		}

	}

	pub fn should_quit(&self) -> bool {
		self.should_quit
	}
}
