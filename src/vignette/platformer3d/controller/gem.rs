use toybox::prelude::*;
use toybox::utility::ResourceScopeID;

use crate::vignette::platformer3d::model::{self, scene::GemState};

pub struct GemController {
	chime_sound_id: audio::SoundId,
	gem_sound_mixer: audio::NodeId,
}


impl GemController {
	pub fn new(engine: &mut toybox::Engine, resource_scope_id: ResourceScopeID) -> Result<GemController, Box<dyn Error>> {
		Ok(GemController {
			chime_sound_id: {
				let source = super::load_audio_buffer("assets/chime.ogg")?;
				engine.audio.add_sound(source)
			},

			gem_sound_mixer: engine.audio.update_graph_immediate(|graph| {
				let node_id = graph.add_node(audio::nodes::MixerNode::new(0.5), graph.output_node());
				graph.pin_node_to_scope(node_id, resource_scope_id);
				node_id
			}),
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
							graph.add_node(sampler_node, gem_sound_mixer);
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