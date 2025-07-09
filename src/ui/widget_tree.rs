use crate::prelude::*;
use crate::ui::*;

pub struct Widget {
	pub parent: WidgetId,

	pub layout: WidgetLayout,

	// Includes padding.
	pub rect: Option<Aabb2>,
}

impl Default for Widget {
	fn default() -> Widget {
		Widget {
			parent: WidgetId::ROOT,

			layout: default(),

			rect: None,
		}
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct WidgetId(pub u64);

impl WidgetId {
	pub const ROOT: WidgetId = WidgetId(0);
}


#[derive(Default)]
pub struct WidgetTree {
	pub widgets: HashMap<WidgetId, Widget>,
	pub children: HashMap<WidgetId, SmallVec<[WidgetId; 4]>>,

	pub submission_order: Vec<WidgetId>,
}

impl WidgetTree {
	pub fn clear(&mut self) {
		self.widgets.clear();
		self.children.clear();
		self.submission_order.clear();
	}

	pub fn make_root(&mut self) -> &mut Widget {
		use std::collections::hash_map::Entry;

		self.submission_order.push(WidgetId::ROOT);

		match self.widgets.entry(WidgetId::ROOT) {
			Entry::Occupied(_) => panic!("Root already exists!"),
			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent: WidgetId::ROOT,
					.. default()
				})
			}
		}
	}

	pub fn get_mut(&mut self, id: WidgetId) -> &mut Widget {
		self.widgets.get_mut(&id).expect("Requesting widget not submitted in layout pass")
	}

	pub fn insert(&mut self, id: WidgetId, parent: WidgetId) -> &mut Widget {
		use std::collections::hash_map::Entry;

		assert!(id != WidgetId::ROOT);

		self.submission_order.push(id);

		self.children.entry(parent)
			.or_default()
			.push(id);

		match self.widgets.entry(id) {
			Entry::Occupied(_) => panic!("Conflicting id! {id:?}"),
			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent,
					.. default()
				})
			}
		}
	}
}