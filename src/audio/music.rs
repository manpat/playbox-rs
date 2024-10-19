use super::{
	db_to_linear,
	linear_to_db,
	Control,
	DC_OFFSET,
};

use std::sync::atomic::{Ordering};

#[derive(Default)]
pub struct MusicProvider {
	target_volume: f32,
	volume: f32,

	pub sample_dt: f64,

	osc_phase: f64,
}


impl MusicProvider {
	pub fn new() -> Self {
		Self {
			// TODO(pat.m): not db - too weird
			target_volume: linear_to_db(0.1),
			volume: linear_to_db(DC_OFFSET),

			sample_dt: 0.0,
			osc_phase: 0.0,
		}
	}

	pub fn update(&mut self, ctl: &Control) {
		self.target_volume = ctl.music_volume.load(Ordering::Relaxed);
	}

	pub fn fill(&mut self, buffer: &mut [f32]) {
		// let mut osc_phase = self.osc_phase * 55.0 * std::f64::consts::TAU;
		// let osc_dt = self.sample_dt * 55.0 * std::f64::consts::TAU;

		// let mut gain = db_to_linear(self.volume);

		// for frame in buffer.chunks_exact_mut(2) {
		// 	if self.volume != self.target_volume {
		// 		let diff = self.target_volume - self.volume;

		// 		self.volume += diff * self.sample_dt as f32 * 100.0;

		// 		if diff.abs() < 0.1 {
		// 			self.volume = self.target_volume;
		// 		}

		// 		gain = db_to_linear(self.volume);
		// 	}

		// 	let osc = (osc_phase + self.osc_phase.sin()*30.0).sin();

		// 	let value = osc as f32 * gain;

		// 	frame[0] += value;
		// 	frame[1] += value;

		// 	self.osc_phase += self.sample_dt;
		// 	osc_phase += osc_dt;
		// }

		// self.osc_phase %= std::f64::consts::TAU;
	}
}
