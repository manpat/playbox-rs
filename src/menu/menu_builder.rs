use crate::prelude::*;

use super::MenuPainter;


pub struct MenuBuilder<'mp, 'ctx> {
	input: &'ctx input::System,

	pub painter: &'mp mut MenuPainter,
	pub screen_rect: Aabb2,
	pub content_rect: Aabb2,

	pub cursor: Vec2,
	pub item_spacing: f32,
	pub font_size: u32,
}

impl<'mp, 'ctx> MenuBuilder<'mp, 'ctx> {
	pub fn new(painter: &'mp mut MenuPainter, ctx: &'ctx mut toybox::Context) -> Self {
		let size = ctx.gfx.backbuffer_size();
		let screen_rect = Aabb2::new(Vec2::zero(), size.to_vec2());
		let content_rect = screen_rect.shrink(Vec2::splat(8.0));

		MenuBuilder {
			painter,
			input: &ctx.input,
			screen_rect,
			content_rect,

			cursor: content_rect.min_max_corner() + Vec2::new(8.0, -8.0),
			item_spacing: 8.0,
			font_size: 32,
		}
	}

	pub fn button(&mut self, label: &str) -> bool {
		let padding = Vec2::new(8.0, 2.0);

		let text_rect = self.painter.text_rect(self.font_size, label);

		let button_size = Vec2::new(text_rect.width(), self.font_size as f32) + padding*2.0;
		let button_rect = Aabb2::new(Vec2::zero(), button_size);

		self.cursor.y -= button_size.y;

		let button_rect = button_rect.translate(self.cursor);
		let text_origin = self.cursor + padding;

		self.cursor.y -= self.item_spacing;

		let is_hovered = self.input.mouse_position_pixels()
			.map(|pos| button_rect.contains_point(pos))
			.unwrap_or(false);

		let is_pressed = is_hovered
			&& self.input.button_down(input::MouseButton::Left);

		let bg_color = Color::grey_a(0.1, 0.3);

		let text_color = match (is_pressed, is_hovered) {
			(true, true) => Color::black().with_alpha(0.2),
			(false, true) => Color::black().with_alpha(0.5),
			(_, false) => Color::white(),
		};

		self.painter.rect(button_rect, bg_color);
		self.painter.text(text_origin, self.font_size, label, text_color);

		is_hovered && self.input.button_just_up(input::MouseButton::Left)
	}
}