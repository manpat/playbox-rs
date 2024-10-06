use crate::prelude::*;

use super::{UiPainter, UiPainterWithShared};


pub struct UiBuilder<'mp, 'ctx, L: UiLayout> {
	input: &'ctx input::System,
	pub input_scale_factor: f32,

	pub painter: UiPainterWithShared<'mp, 'ctx>,
	pub font_size: u32,

	pub layout: L,
}

impl<'mp, 'ctx, L: UiLayout> UiBuilder<'mp, 'ctx, L> {
	pub fn new(painter: &'mp mut UiPainter, ctx: &'ctx Context<'_>, layout: L) -> Self {
		UiBuilder {
			painter: painter.with_shared(ctx.ui_shared),
			input: &ctx.input,
			input_scale_factor: 1.0,

			font_size: 16,

			layout,
		}
	}

	pub fn with_layout<'s, L2: UiLayout>(&'s mut self, new_layout: L2) -> UiBuilder<'s, 'ctx, L2> {
		UiBuilder {
			input: self.input,
			input_scale_factor: self.input_scale_factor,

			painter: self.painter.painter.with_shared(self.painter.shared),
			font_size: self.font_size,

			layout: new_layout,
		}
	}

	pub fn button(&mut self, label: &str) -> bool {
		let padding = Vec2::new(8.0, 2.0);

		let text_rect = self.painter.text_rect(self.font_size, label);

		let button_size = Vec2::new(text_rect.width(), self.font_size as f32) + padding*2.0;
		let button_rect = self.layout.allocate(button_size);

		let text_origin = button_rect.min + padding;

		let is_hovered = self.input.mouse_position_pixels()
			.map(|pos| button_rect.contains_point(pos * self.input_scale_factor))
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