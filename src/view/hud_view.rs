use crate::prelude::*;
use model::*;

pub const HUD_FRAME_STAGE: gfx::FrameStage = gfx::FrameStage::Ui(0);


pub struct HudView {
	_message_bus: MessageBus,
}

impl HudView {
	pub fn new(message_bus: MessageBus) -> anyhow::Result<Self> {
		Ok(HudView {
			_message_bus: message_bus,
		})
	}

	pub fn draw(&mut self, gfx: &mut gfx::System, model: &Model) {
		let screen_size = gfx.backbuffer_size().to_vec2();
		let screen_bounds = Aabb2::from_min_size(Vec2::zero(), screen_size/2.0);

		let mut usable_area = screen_bounds.shrink(2.0);


		let mut quads = vec![
			usable_area.cut_top(12.0).shrink(2.0),
			usable_area.cut_bottom(24.0).shrink(2.0),
			usable_area.cut_left(24.0).shrink(2.0),
			usable_area.cut_right(24.0).shrink(2.0),
			usable_area.cut_top(12.0).shrink(2.0),
		];

		if model.interactions.can_interact {
			quads.push(Aabb2::from_center_extents(usable_area.center(), 8.0));
			// TODO(pat.m): text info about hovered interactable
		}

		let mut hud_group = gfx.frame_encoder.command_group(HUD_FRAME_STAGE);
		let indices = hud_group.upload(&[0, 1, 2, 0, 2, 3]);

		let projection = Mat4::ortho(
			screen_bounds.min.x, screen_bounds.max.x,
			screen_bounds.min.y, screen_bounds.max.y,
			-1.0, 1.0
		);
		hud_group.bind_shared_ubo(0, &[projection]);


		for quad in quads {
			let color = Color::white().with_alpha(0.02);
			let vertices = [
				gfx::StandardVertex::with_color(quad.min.floor().extend(0.0), color),
				gfx::StandardVertex::with_color(quad.min_max_corner().floor().extend(0.0), color),
				gfx::StandardVertex::with_color(quad.max.floor().extend(0.0), color),
				gfx::StandardVertex::with_color(quad.max_min_corner().floor().extend(0.0), color),
			];

			hud_group.draw(gfx::CommonShader::StandardVertex, gfx::CommonShader::FlatTexturedFragment)
				.ssbo(0, &vertices)
				.indexed(indices)
				.elements(6)
				.blend_mode(gfx::BlendMode::ALPHA);
		}
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