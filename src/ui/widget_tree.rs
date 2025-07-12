use crate::prelude::*;
use crate::ui::*;

use std::mem::MaybeUninit;
use std::pin::Pin;

pub struct Widget {
	pub parent: WidgetId,

	pub layout: *mut WidgetLayout,

	// Includes padding.
	pub rect: Option<Aabb2>,

	pub epoch: u8,
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

	epoch: u8,
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

			epoch: 0,
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

	pub fn reset(&mut self) {
		self.submission_order.clear();

		self.next_layout_index = 0;
		self.epoch = self.epoch.wrapping_add(1);

		self.track_widget(WidgetId::ROOT, WidgetId::ROOT);
	}

	pub fn track_widget(&mut self, id: WidgetId, parent: WidgetId) -> &mut Widget {
		use std::collections::hash_map::Entry;

		let layout = self.alloc_layout();
		self.submission_order.push(id);

		if id != WidgetId::ROOT {
			self.children.entry(parent)
				.or_default()
				.push(id);
		}

		self.children.entry(id)
			.or_default()
			.clear();

		match self.widgets.entry(id) {
			Entry::Occupied(entry) => {
				let widget = entry.into_mut();
				assert!(widget.epoch != self.epoch, "Id conflict!");
				widget.parent = parent;
				widget.layout = layout;
				widget.epoch = self.epoch;
				widget
			},

			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent,
					rect: None,
					layout,
					epoch: self.epoch
				})
			}
		}
	}

	pub fn widget_count(&self) -> usize {
		self.widgets.len()
	}

	pub fn get_layout(&self, id: WidgetId) -> WidgetLayout {
		match self.widgets.get(&id) {
			Some(widget) if widget.epoch == self.epoch => unsafe{ widget.layout.read() },
			_ => default(),
		}
	}

	pub fn get_children(&self, id: WidgetId) -> &[WidgetId] {
		self.children.get(&id)
			.map_or(&[], |v| v)
	}

	pub fn get_mut(&mut self, id: WidgetId) -> &mut Widget {
		self.widgets.get_mut(&id).expect("Requesting widget not yet submitted")
	}
}