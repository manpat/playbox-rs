use crate::prelude::*;

use model::Placement;


#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Object {
	pub name: String,

	#[serde(flatten)]
	pub placement: Placement,

	pub info: ObjectInfo,

	// appearance
	// interactions
	// - how can it be interacted with
	// 		- basic button interaction? does it need to be looked at or just in area
	// 		- use an item on it?
	// - can it give an item?
	// - can it display text?
	// - does it have a name?
	// - does it take you somewhere else
	// - does it trigger effects
	// - can it be interacted with multiple ways
}


#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum ObjectInfo {
	Debug,

	Ladder {
		target_world: String,
		target_object: String,
	},

	Light {
		color: Color,
		height: f32,
		power: f32,
	},

	Chest {
		// content: Item,
	},

	Npc,
}
