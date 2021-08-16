use toybox::prelude::*;

use crate::model::{self, scene::GemState};

pub struct GemController {
	chime_sound: audio::SoundAssetID,
}


impl GemController {
	pub fn new(audio: &mut audio::AudioSystem) -> Result<GemController, Box<dyn Error>> {
		Ok(GemController {
			chime_sound: {
				let source = audio::FileStream::from_vorbis_file("assets/chime.ogg")?;
				audio.register_file_stream(source)
			}
		})
	}

	pub fn update(&mut self, audio: &mut audio::AudioSystem, scene: &mut model::Scene, player: &model::Player) {
		let ply_pos = player.position;

		for gem in scene.gems.iter_mut() {
			match gem.state {
				GemState::Idle => {
					let dist = (gem.position - ply_pos).length();
					if dist < 2.5 {
						gem.state = GemState::Collecting(0.0);
						audio.play_one_shot(self.chime_sound);
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