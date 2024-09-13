use crate::prelude::*;
use model::*;

pub const HUD_FRAME_STAGE: gfx::FrameStage = gfx::FrameStage::Ui(0);


pub struct HudView {
	_message_bus: MessageBus,
	painter: menu::MenuPainter,
}

impl HudView {
	pub fn new(gfx: &mut gfx::System, message_bus: MessageBus) -> anyhow::Result<Self> {
		Ok(HudView {
			_message_bus: message_bus,
			painter: menu::MenuPainter::new(gfx, HUD_FRAME_STAGE)?,
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System, model: &Model) {
		let screen_size = gfx.backbuffer_size().to_vec2();
		let screen_bounds = Aabb2::from_min_size(Vec2::zero(), screen_size/2.0);

		let mut usable_area = screen_bounds.shrink(2.0);

		let text_size = self.painter.text_rect(16, "Testing 123").size();

		self.painter.text(usable_area.min, 16, "Testing 123", Color::white());
		self.painter.text(usable_area.max - text_size, 16, "Testing 123", Color::white());
		self.painter.text(usable_area.max_min_corner() - Vec2::from_x(text_size.x), 16, "Testing 123", Color::white());
		self.painter.text(usable_area.min_max_corner() - Vec2::from_y(text_size.y), 16, "Testing 123", Color::white());


		if let Some(object) = model.interactions.hovered_object.and_then(|idx| model.world.objects.get(idx)) {
			self.painter.text(usable_area.center(), 16, format!("{object:#?}"), Color::white());
		}


		let quads = [
			usable_area.cut_top(12.0).shrink(2.0),
			usable_area.cut_bottom(24.0).shrink(2.0),
			usable_area.cut_left(24.0).shrink(2.0),
			usable_area.cut_right(24.0).shrink(2.0),
			usable_area.cut_top(12.0).shrink(2.0),
		];

		for quad in quads {
			let color = Color::white().with_alpha(0.02);
			self.painter.rect(quad, color);
		}


		self.painter.submit(gfx, screen_bounds);
	}
}







trait Aabb2UIExt {
	fn with_left(&self, new: f32) -> Aabb2;
	fn with_right(&self, new: f32) -> Aabb2;
	fn with_top(&self, new: f32) -> Aabb2;
	fn with_bottom(&self, new: f32) -> Aabb2;

	fn cut_left(&mut self, amount: f32) -> Aabb2;
	fn cut_right(&mut self, amount: f32) -> Aabb2;
	fn cut_top(&mut self, amount: f32) -> Aabb2;
	fn cut_bottom(&mut self, amount: f32) -> Aabb2;
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
}