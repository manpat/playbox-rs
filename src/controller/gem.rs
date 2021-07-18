
use crate::model::{self, scene::GemState};


pub struct GemController {

}


impl GemController {
	pub fn new() -> GemController {
		GemController {

		}
	}

	pub fn update(&mut self, player: &model::Player, scene: &mut model::Scene) {
		let ply_pos = player.position;

		for gem in scene.gems.iter_mut() {
			match gem.state {
				GemState::Idle => {
					let dist = (gem.position - ply_pos).to_xz().length();
					if dist < 1.5 {
						gem.state = GemState::Collecting(0.0);
						println!("GEM");
					}
				}

				GemState::Collecting(t) => if t >= 1.0 {
					gem.state = GemState::Collected;
				} else {
					gem.state = GemState::Collecting(t + 1.0/60.0);
				}

				GemState::Collected => {}
			}
		}
	}
}