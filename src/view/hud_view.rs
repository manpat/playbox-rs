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

		let usable_area = screen_bounds.shrink(8.0);

		let text_size = self.painter.text_rect(16, "Testing 123").size();

		self.painter.text(usable_area.min, 16, "Testing 123", Color::white());
		self.painter.text(usable_area.max - text_size, 16, "Testing 123", Color::white());
		self.painter.text(usable_area.max_min_corner() - text_size.to_x0(), 16, "Testing 123", Color::white());
		self.painter.text(usable_area.min_max_corner() - text_size.to_0y(), 16, "Testing 123", Color::white());


		if let Some(object) = model.interactions.hovered_object.and_then(|idx| model.world.objects.get(idx)) {
			let interact_message = match &object.info {
				ObjectInfo::Ladder { target_world, .. } => format!("To {target_world}"),
				_ => format!("Frob '{}'", object.name),
			};

			let text_size = self.painter.text_rect(16, &interact_message).size();

			self.painter.rect(Aabb2::from_center_extents(usable_area.center(), 8.0), Color::grey(0.5).with_alpha(0.1));
			// self.painter.text(usable_area.center() - text_size * Vec2::new(0.5, 1.0) - Vec2::from_y(12.0), 16, interact_message, Color::grey(0.5));
			self.painter.text(usable_area.center() - text_size.to_0y()/2.0 + Vec2::from_x(12.0), 16, interact_message, Color::grey(0.5));
		}


		self.painter.submit(gfx, screen_bounds);
	}
}






#[allow(dead_code)]
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