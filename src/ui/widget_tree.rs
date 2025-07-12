use crate::prelude::*;
use crate::ui::*;

use std::mem::MaybeUninit;
use std::pin::Pin;

pub struct Widget {
	pub parent: WidgetId,

	pub layout: *mut WidgetLayout,

	// Includes padding.
	pub rect: Aabb2,

	pub epoch: u8,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct WidgetId(pub u64);

impl WidgetId {
	pub const ROOT: WidgetId = WidgetId(0);
}



pub struct WidgetStackEntry {
	pub id: WidgetId,
	pub num_children: u32,
}


pub struct WidgetTree {
	pub widgets: HashMap<WidgetId, Widget>,
	pub children: HashMap<WidgetId, SmallVec<[WidgetId; 4]>>,

	// TODO(pat.m): some kind of dynamic structure/arena
	pub layout_storage: Pin<Box<[MaybeUninit<WidgetLayout>]>>,
	pub next_layout_index: usize,

	pub submission_order: Vec<WidgetId>,
	pub submission_stack: Vec<WidgetStackEntry>,

	gc_epoch: u8,
	epoch: u8,
}

impl WidgetTree {
	pub fn new() -> WidgetTree {
		WidgetTree {
			widgets: default(),
			children: default(),
			submission_order: default(),
			submission_stack: default(),

			// 10k widgets oughta be enough for anybody...
			layout_storage: Pin::new(Box::new_uninit_slice(10000)),
			next_layout_index: 0,

			gc_epoch: 0,
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

	pub fn garbage_collect(&mut self) -> Vec<WidgetId> {
		let mut removed_widgets = Vec::new();
		self.widgets.retain(|&id, widget| {
			let epoch_diff = widget.epoch.wrapping_sub(self.gc_epoch).cast_signed();
			let has_expired = epoch_diff < 0;
			if has_expired {
				log::info!("WIDGET {id:?} GC'D");
				removed_widgets.push(id);
			}

			!has_expired
		});

		for id in removed_widgets.iter() {
			self.children.remove(id);
		}

		self.gc_epoch = self.epoch;

		removed_widgets
	}

	pub fn reset(&mut self) {
		self.submission_order.clear();
		self.submission_stack.clear();

		self.next_layout_index = 0;
		self.epoch = self.epoch.wrapping_add(1);

		self.track_widget_start(WidgetId::ROOT);
	}

	pub fn track_widget_start(&mut self, id: WidgetId) -> &mut Widget {
		use std::collections::hash_map::Entry;

		let layout = self.alloc_layout();
		let parent_id;

		if id != WidgetId::ROOT {
			let parent_entry = self.submission_stack.last_mut().unwrap();
			parent_entry.num_children += 1;
			parent_id = parent_entry.id;

			self.children.entry(parent_id)
				.or_default()
				.push(id);
		} else {
			parent_id = WidgetId::ROOT;
		}

		self.submission_order.push(id);
		self.submission_stack.push(WidgetStackEntry{ id, num_children: 0 });

		self.children.entry(id)
			.or_default()
			.clear();

		match self.widgets.entry(id) {
			Entry::Occupied(entry) => {
				let widget = entry.into_mut();
				assert!(widget.epoch != self.epoch, "Id conflict!");
				widget.parent = parent_id;
				widget.layout = layout;
				widget.epoch = self.epoch;
				widget
			},

			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent: parent_id,
					rect: Aabb2::zero(),
					layout,
					epoch: self.epoch
				})
			}
		}
	}

	pub fn track_widget_end(&mut self) {
		self.submission_stack.pop();
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