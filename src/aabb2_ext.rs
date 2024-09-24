use crate::prelude::*;

pub trait Aabb2UIExt {
	fn with_left(&self, new: f32) -> Aabb2;
	fn with_right(&self, new: f32) -> Aabb2;
	fn with_top(&self, new: f32) -> Aabb2;
	fn with_bottom(&self, new: f32) -> Aabb2;

	fn cut_left(&mut self, amount: f32) -> Aabb2;
	fn cut_right(&mut self, amount: f32) -> Aabb2;
	fn cut_top(&mut self, amount: f32) -> Aabb2;
	fn cut_bottom(&mut self, amount: f32) -> Aabb2;

	fn floor(&self) -> Aabb2;
}

impl Aabb2UIExt for Aabb2 {
	fn with_left(&self, new: f32) -> Aabb2 {
		Aabb2 { min: Vec2 { x: new, ..self.min }, ..*self }
	}
	fn with_right(&self, new: f32) -> Aabb2 {
		Aabb2 { max: Vec2 { x: new, ..self.max }, ..*self }
	}
	fn with_bottom(&self, new: f32) -> Aabb2 {
		Aabb2 { min: Vec2 { y: new, ..self.min }, ..*self }
	}
	fn with_top(&self, new: f32) -> Aabb2 {
		Aabb2 { max: Vec2 { y: new, ..self.max }, ..*self }
	}

	fn cut_left(&mut self, amount: f32) -> Aabb2 {
		let mid_x = (self.min.x + amount).min(self.max.x);

		let left = self.with_right(mid_x);
		*self = self.with_left(mid_x);

		left
	}

	fn cut_right(&mut self, amount: f32) -> Aabb2 {
		let mid_x = (self.max.x - amount).max(self.min.x);

		let right = self.with_left(mid_x);
		*self = self.with_right(mid_x);

		right
	}

	fn cut_bottom(&mut self, amount: f32) -> Aabb2 {
		let mid_y = (self.min.y + amount).min(self.max.y);

		let bottom = self.with_top(mid_y);
		*self = self.with_bottom(mid_y);

		bottom
	}

	fn cut_top(&mut self, amount: f32) -> Aabb2 {
		let mid_y = (self.max.y - amount).max(self.min.y);

		let top = self.with_bottom(mid_y);
		*self = self.with_top(mid_y);

		top
	}

	fn floor(&self) -> Aabb2 {
		Aabb2 {
			min: self.min.floor(),
			max: self.max.floor(),
		}
	}
}