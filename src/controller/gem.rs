use toybox::prelude::*;

use crate::model::{self, scene::GemState};

pub struct GemController {
	chime_sound: audio::SoundAssetID,
	gem_sound_bus: audio::BusID,
}


impl GemController {
	pub fn new(engine: &mut toybox::Engine) -> Result<GemController, Box<dyn Error>> {
		Ok(GemController {
			chime_sound: {
				let source = audio::FileStream::from_vorbis_file("assets/chime.ogg")?;
				engine.audio.register_file_stream(source)
			},

			gem_sound_bus: {
				let bus = engine.audio.new_bus("Gems");
				engine.audio.get_bus_mut(bus).unwrap().set_gain(0.5);
				bus
			},
		})
	}

	pub fn update(&mut self, engine: &mut toybox::Engine, scene: &mut model::Scene, player: &model::Player) {
		for gem in scene.gems.iter_mut() {
			match gem.state {
				GemState::Idle => {
					let dist = (gem.position - player.position).length();
					if dist < 2.5 {
						gem.state = GemState::Collecting(0.0);
						engine.audio.start_sound(self.gem_sound_bus, self.chime_sound);
					}
				}

				GemState::Collecting(t) => if t >= 1.0 {
					gem.state = GemState::Collected;
				} else {
					gem.state = GemState::Collecting(t + 2.0/60.0);
				}

				GemState::Collected => {}
			}
		}
	}
}