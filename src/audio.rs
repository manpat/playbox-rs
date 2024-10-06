use toybox::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};


#[derive(Clone)]
pub struct MyAudioSystem {
	control: Arc<Control>,
}

impl MyAudioSystem {
	pub fn start(audio: &mut audio::System) -> anyhow::Result<MyAudioSystem> {
		let control = Arc::new(Control::new());

		let provider = MyAudioProvider {
			control: control.clone(),

			music: MusicProvider::default(),
			sfx: SfxProvider::new(),
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

	music: MusicProvider,
	sfx: SfxProvider,
}

impl audio::Provider for MyAudioProvider {
	fn on_configuration_changed(&mut self, config: Option<audio::Configuration>) {
		let Some(config) = config else { return };

		self.sfx.sample_dt = 1.0/config.sample_rate as f64;
		log::info!("Configuration change! dt = {}", self.sfx.sample_dt);

		assert!(config.channels == 2);
	}

	fn fill_buffer(&mut self, buffer: &mut [f32]) {
		buffer.fill(0.0);

		self.sfx.update(&self.control);
		self.sfx.fill(buffer);
	}
}


#[derive(Default)]
struct MusicProvider {}

struct SfxProvider {
	target_volume: f32,
	volume: f32,

	sample_dt: f64,

	osc_phase: f64,
	env_phase: f64,
}

impl SfxProvider {
	fn new() -> Self {
		Self {
			// TODO(pat.m): not db - too weird
			target_volume: linear_to_db(0.3),
			volume: linear_to_db(DC_OFFSET),

			sample_dt: 0.0,
			osc_phase: 0.0,
			env_phase: 1.0,
		}
	}

	fn update(&mut self, ctl: &Control) {
		if ctl.trigger_sfx.fetch_and(false, Ordering::Relaxed) {
			self.env_phase = 0.0;
		}

		self.target_volume = ctl.sfx_volume.load(Ordering::Relaxed);
	}

	fn fill(&mut self, buffer: &mut [f32]) {
		let mut osc_phase = self.osc_phase * 110.0 * std::f64::consts::TAU;
		let osc_dt = self.sample_dt * 110.0 * std::f64::consts::TAU;

		let mut gain = db_to_linear(self.volume);

		for frame in buffer.chunks_exact_mut(2) {
			if self.volume != self.target_volume {
				let diff = self.target_volume - self.volume;

				self.volume += diff * self.sample_dt as f32 * 100.0;

				if diff.abs() < 0.1 {
					self.volume = self.target_volume;
				}

				gain = db_to_linear(self.volume);
			}

			let osc = osc_phase.sin();
			let amp = (1.0 - self.env_phase).max(0.0).powi(2) * gain as f64;

			let value = (amp * osc) as f32;

			frame[0] += value;
			frame[1] += value;

			self.osc_phase += self.sample_dt;
			self.env_phase += self.sample_dt * 2.0;
			osc_phase += osc_dt;
		}

		self.osc_phase %= std::f64::consts::TAU;
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