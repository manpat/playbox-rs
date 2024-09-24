use crate::prelude::*;

pub trait Layout {
	fn allocate(&mut self, size: Vec2) -> Aabb2;
}

pub struct DumbLayout {
	pub content_rect: Aabb2,
	pub cursor: Vec2,
	pub item_spacing: f32,
}

impl DumbLayout {
	pub fn new(content_rect: Aabb2) -> Self {
		DumbLayout {
			content_rect,

			cursor: content_rect.min_max_corner() + Vec2::new(8.0, -8.0),
			item_spacing: 8.0,
		}
	}
}

impl Layout for DumbLayout {
	fn allocate(&mut self, size: Vec2) -> Aabb2 {
		self.cursor.y -= size.y;
		let rect = Aabb2::from_min_size(self.cursor, size);
		self.cursor.y -= self.item_spacing;
		rect
	}
}