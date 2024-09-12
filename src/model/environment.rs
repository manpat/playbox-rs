use crate::prelude::*;
use model::*;

#[derive(Debug)]
pub struct EnvironmentModel {
	pub fog: FogParameters,
}

impl EnvironmentModel {
	pub fn new(world: &World, _message_bus: &MessageBus) -> Self {
		EnvironmentModel {
			fog: world.fog,
		}
	}

	pub fn update(&mut self, world: &World, _message_bus: &MessageBus) {
		self.fog = world.fog;
	}
}



#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FogParameters {
	pub color: Color,
	pub start: f32,
	pub distance: f32,
	pub emission: f32,

	pub transparency: f32,
}

impl Default for FogParameters {
	fn default() -> Self {
		FogParameters {
			color: Color::white(),
			start: 0.0,
			distance: 30.0,
			emission: 1.0,

			transparency: 0.5,
		}
	}
}