use crate::prelude::*;

pub trait UiLayout {
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

impl UiLayout for DumbLayout {
	fn allocate(&mut self, size: Vec2) -> Aabb2 {
		let rect = self.available_rect.cut_top(size.y);
		self.available_rect.max.y -= self.item_spacing;
		rect
	}
}



pub struct HorizontalLayout {
	pub available_rect: Aabb2,
	pub item_spacing: f32,
}

impl HorizontalLayout {
	pub fn new(content_rect: Aabb2) -> Self {
		HorizontalLayout {
			available_rect: content_rect.floor(),
			item_spacing: 8.0,
		}
	}
}

impl UiLayout for HorizontalLayout {
	fn allocate(&mut self, size: Vec2) -> Aabb2 {
		let mut rect = self.available_rect.cut_left(size.x);
		self.available_rect.min.x += self.item_spacing;
		rect.max.y = rect.min.y + rect.height().min(size.y);
		rect
	}
}