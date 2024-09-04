use crate::prelude::*;

use model::Placement;


#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Object {
	pub name: String,

	#[serde(flatten)]
	pub placement: Placement,

	#[serde(flatten)]
	pub kind: ObjectInfo,

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


#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ObjectInfo {
	Ladder {
		target_world: String,
		target_object: String,
	},

	Chest {
		// content: Item,
	},

	Npc,
}

