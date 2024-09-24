use crate::prelude::*;

pub trait Layout {
	fn allocate(&mut self, size: Vec2) -> Aabb2;
}

pub struct DumbLayout {
	pub available_rect: Aabb2,
	pub item_spacing: f32,
}

impl DumbLayout {
	pub fn new(content_rect: Aabb2) -> Self {
		DumbLayout {
			available_rect: content_rect.floor(),
			item_spacing: 8.0,
		}
	}
}

impl Layout for DumbLayout {
	fn allocate(&mut self, size: Vec2) -> Aabb2 {
		let rect = self.available_rect.cut_top(size.y);
		self.available_rect.max.y -= self.item_spacing;
		rect
	}
}