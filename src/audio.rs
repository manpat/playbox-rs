use toybox::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};


pub struct MyAudioSystem {
	trigger: Arc<AtomicBool>,
}

impl MyAudioSystem {
	pub fn start(audio: &mut audio::System) -> anyhow::Result<MyAudioSystem> {
		let trigger = Arc::new(AtomicBool::new(false));

		let provider = MyAudioProvider {
			trigger: trigger.clone(),
			sample_dt: 0.0,
			osc_phase: 0.0,
			env_phase: 1.0,
		};

		audio.set_provider(provider)?;

		Ok(MyAudioSystem {
			trigger
		})
	}

	pub fn trigger(&self) {
		self.trigger.store(true, Ordering::Relaxed);
	}
}


#[derive(Default)]
struct MyAudioProvider {
	trigger: Arc<AtomicBool>,
	sample_dt: f64,

	osc_phase: f64,
	env_phase: f64,
}

impl audio::Provider for MyAudioProvider {
	fn on_configuration_changed(&mut self, config: audio::Configuration) {
		self.sample_dt = 1.0/config.sample_rate as f64;
		assert!(config.channels == 2);
	}

	fn fill_buffer(&mut self, buffer: &mut [f32]) {
		let mut osc_phase = self.osc_phase * 220.0 * std::f64::consts::TAU;
		let osc_dt = self.sample_dt * 220.0 * std::f64::consts::TAU;

		if self.trigger.fetch_and(false, Ordering::Relaxed) {
			self.env_phase = 0.0;
		}

		for frame in buffer.chunks_exact_mut(2) {
			let osc = osc_phase.sin();
			let amp = (1.0 - self.env_phase).max(0.0).powi(2) * 0.3;

			let value = (amp * osc) as f32;

			frame[0] = value;
			frame[1] = value;

			self.osc_phase += self.sample_dt;
			self.env_phase += self.sample_dt * 2.0;
			osc_phase += osc_dt;
		}

		self.osc_phase %= std::f64::consts::TAU;
	}
}
