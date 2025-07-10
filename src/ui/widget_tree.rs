use crate::prelude::*;
use crate::ui::*;

use std::mem::MaybeUninit;
use std::pin::Pin;

pub struct Widget {
	pub parent: WidgetId,

	pub layout: *mut WidgetLayout,

	// Includes padding.
	pub rect: Option<Aabb2>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct WidgetId(pub u64);

impl WidgetId {
	pub const ROOT: WidgetId = WidgetId(0);
}


pub struct WidgetTree {
	pub widgets: HashMap<WidgetId, Widget>,
	pub children: HashMap<WidgetId, SmallVec<[WidgetId; 4]>>,

	// TODO(pat.m): some kind of dynamic structure/arena
	pub layout_storage: Pin<Box<[MaybeUninit<WidgetLayout>]>>,
	pub next_layout_index: usize,

	pub submission_order: Vec<WidgetId>,
}

impl WidgetTree {
	pub fn new() -> WidgetTree {
		WidgetTree {
			widgets: default(),
			children: default(),
			submission_order: default(),

			// 10k widgets oughta be enough for anybody...
			layout_storage: Pin::new(Box::new_uninit_slice(10000)),
			next_layout_index: 0,
		}
	}

	fn alloc_layout(&mut self) -> *mut WidgetLayout {
		let index = self.next_layout_index;
		assert!(index < self.layout_storage.len());

		self.next_layout_index += 1;

		unsafe {
			let ptr = self.layout_storage[index].as_mut_ptr();
			ptr.write(default());
			ptr
		}
	}

	pub fn clear(&mut self) {
		self.widgets.clear();
		self.children.clear();
		self.submission_order.clear();

		self.next_layout_index = 0;
	}

	pub fn make_root(&mut self) -> &mut Widget {
		use std::collections::hash_map::Entry;

		let layout = self.alloc_layout();

		self.submission_order.push(WidgetId::ROOT);

		match self.widgets.entry(WidgetId::ROOT) {
			Entry::Occupied(_) => panic!("Root already exists!"),
			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent: WidgetId::ROOT,
					rect: None,
					layout,
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

		let layout = self.alloc_layout();
		self.submission_order.push(id);
		self.children.entry(parent)
			.or_default()
			.push(id);

		match self.widgets.entry(id) {
			Entry::Occupied(_) => panic!("Conflicting id! {id:?}"),
			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent,
					rect: None,
					layout,
				})
			}
		}
	}
}