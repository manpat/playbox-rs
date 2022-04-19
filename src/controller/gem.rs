use toybox::prelude::*;

use crate::model::{self, scene::GemState};

pub struct GemController {
	chime_sound_id: audio::SoundId,
	gem_sound_mixer: audio::NodeId,
}


impl GemController {
	pub fn new(engine: &mut toybox::Engine) -> Result<GemController, Box<dyn Error>> {
		Ok(GemController {
			chime_sound_id: {
				let source = super::load_audio_buffer("assets/chime.ogg")?;
				engine.audio.add_sound(source)
			},

			gem_sound_mixer: {
				engine.audio.add_node_with_send(audio::nodes::MixerNode::new(0.5), engine.audio.output_node())
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

						let gem_sound_mixer = self.gem_sound_mixer;
						let sampler_node = audio::nodes::SamplerNode::new(self.chime_sound_id);

						engine.audio.queue_update(move |graph| {
							let sampler_id = graph.add_node(sampler_node, false);
							graph.add_send(sampler_id, gem_sound_mixer);
						});
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