use toybox::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

mod sfx;
mod music;



// TODO(pat.m): noise + filters for ambience
// TODO(pat.m): basic sequencer for music
// TODO(pat.m): voice based synthesizer for playing sequences
// TODO(pat.m): audio loader and sample buffer manager, so we can play sound files as sfx
// TODO(pat.m): basic delay/reverb
// TODO(pat.m): parameterisation/controls for all of the above
// TODO(pat.m): spatialisation - how to handle spatial objects?


pub struct MyAudioSystem {
	control: Arc<Control>,
}

impl MyAudioSystem {
	pub fn start(audio: &mut audio::System) -> anyhow::Result<MyAudioSystem> {
		// TODO(pat.m): load from config
		let control = Arc::new(Control::new());

		let provider = MyAudioProvider {
			control: control.clone(),

			music: music::MusicProvider::default(),
			sfx: sfx::SfxProvider::new(),
		};

		audio.set_provider(provider)?;

		Ok(MyAudioSystem { control })
	}

	pub fn trigger(&self) {
		self.control.trigger_sfx.store(true, Ordering::Relaxed);
	}
}


struct Control {
	trigger_sfx: AtomicBool,

	music_volume: AtomicF32,
	sfx_volume: AtomicF32,
}

impl Control {
	fn new() -> Self {
		let initial_volume = linear_to_db(0.1);

		Control {
			trigger_sfx: AtomicBool::new(false),
			music_volume: AtomicF32::new(initial_volume),
			sfx_volume: AtomicF32::new(initial_volume),
		}
	}
}



struct MyAudioProvider {
	control: Arc<Control>,

	music: music::MusicProvider,
	sfx: sfx::SfxProvider,
}

impl audio::Provider for MyAudioProvider {
	fn on_configuration_changed(&mut self, config: Option<audio::Configuration>) {
		let Some(config) = config else { return };

		let sample_dt = 1.0/config.sample_rate as f64;

		self.sfx.sample_dt = sample_dt;
		self.music.sample_dt = sample_dt;

		log::info!("Configuration change! dt = {sample_dt}");

		assert!(config.channels == 2);
	}

	fn fill_buffer(&mut self, buffer: &mut [f32]) {
		buffer.fill(0.0);

		self.sfx.update(&self.control);
		self.sfx.fill(buffer);

		self.music.update(&self.control);
		self.music.fill(buffer);

		// TODO(pat.m): compress/limit
	}
}


const DC_OFFSET: f32 = 1.0E-25;

fn linear_to_db(lin: f32) -> f32 {
	lin.ln() * 20.0 / std::f32::consts::LN_10
}

fn db_to_linear(db: f32) -> f32 {
	(db * std::f32::consts::LN_10 / 20.0).exp()
}



struct AtomicF32(AtomicU32);

impl AtomicF32 {
	fn new(f: f32) -> Self {
		AtomicF32(AtomicU32::new(f.to_bits()))
	}

	fn store(&self, f: f32, ordering: Ordering) {
		self.0.store(f.to_bits(), ordering)
	}

	fn load(&self, ordering: Ordering) -> f32 {
		f32::from_bits(self.0.load(ordering))
	}
}