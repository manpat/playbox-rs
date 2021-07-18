use toybox::prelude::*;

#[derive(Debug)]
pub struct Scene {
	pub source_data: toy::Project,
	pub gems: Vec<Gem>,
}

impl Scene {
	pub fn new() -> Result<Scene, Box<dyn Error>> {
		let scene_data = std::fs::read("assets/scene.toy")?;
		let source_data = toy::load(&scene_data)?;

		let scene = source_data.find_scene("main").unwrap();
		let gems = scene.entities()
			.filter(|e| e.name.starts_with("GEM_"))
			.map(|e| Gem {
				position: e.position,
				state: GemState::Idle,
			})
			.collect();

		Ok(Scene {
			source_data,
			gems,
		})
	}
}


#[derive(Debug, Copy, Clone)]
pub enum GemState {
	Idle,
	Collecting(f32),
	Collected,
}

#[derive(Debug, Copy, Clone)]
pub struct Gem {
	pub position: Vec3,
	pub state: GemState,
}