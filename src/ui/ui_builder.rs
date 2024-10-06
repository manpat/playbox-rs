use crate::prelude::*;

use super::{UiPainter, UiPainterWithShared, Layout};


pub struct UiBuilder<'mp, 'ctx, L: Layout> {
	input: &'ctx input::System,

	pub painter: UiPainterWithShared<'mp, 'ctx>,
	pub font_size: u32,

	pub layout: L,
}

impl<'mp, 'ctx, L: Layout> UiBuilder<'mp, 'ctx, L> {
	pub fn new(painter: &'mp mut UiPainter, ctx: &'ctx Context<'_>, layout: L) -> Self {
		UiBuilder {
			painter: painter.with_shared(ctx.ui_shared),
			input: &ctx.input,

			font_size: 16,

			layout,
		}
	}

	pub fn button(&mut self, label: &str) -> bool {
		let padding = Vec2::new(8.0, 2.0);

		let text_rect = self.painter.text_rect(self.font_size, label);

		let button_size = Vec2::new(text_rect.width(), self.font_size as f32) + padding*2.0;
		let button_rect = self.layout.allocate(button_size);

		let text_origin = button_rect.min + padding;

		let is_hovered = self.input.mouse_position_pixels()
			.map(|pos| button_rect.contains_point(pos/2.0)) // TODO(pat.m): magic scaling number!
			.unwrap_or(false);

		let is_pressed = is_hovered
			&& self.input.button_down(input::MouseButton::Left);

		// TODO(pat.m): style
		let bg_color = Color::grey_a(0.05, 0.3);

		let text_color = match (is_pressed, is_hovered) {
			(true, true) => Color::rgb(0.5, 0.05, 0.05),
			(false, true) => Color::rgb(0.4, 0.01, 0.01),
			(_, false) => Color::white(),
		};

		self.painter.rect(button_rect, bg_color);
		self.painter.text(text_origin, self.font_size, label, text_color);

		is_hovered && self.input.button_just_up(input::MouseButton::Left)
	}
}