use toybox::prelude::*;

#[derive(Debug)]
pub struct Scene {
	pub source_data: toy::Project,
	pub main_scene_name: String,
	pub gems: Vec<Gem>,
}

impl Scene {
	pub fn new(scene_path: impl AsRef<std::path::Path>, main_scene_name: impl Into<String>) -> Result<Scene, Box<dyn Error>> {
		let scene_data = std::fs::read(scene_path)?;
		let source_data = toy::load(&scene_data)?;

		let main_scene_name = main_scene_name.into();

		let scene = source_data.find_scene(&main_scene_name).expect("Failed to find main scene");
		let gems = scene.entities()
			.filter(|e| e.name.starts_with("GEM_"))
			.map(|e| Gem {
				position: e.position,
				state: GemState::Idle,
			})
			.collect();

		Ok(Scene {
			source_data,
			main_scene_name,
			gems,
		})
	}

	pub fn main_scene(&self) -> toy::SceneRef<'_> {
		self.source_data.find_scene(&self.main_scene_name)
			.expect("missing main scene")
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