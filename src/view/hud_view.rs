use crate::prelude::*;
use model::*;

pub struct HudView {
	_message_bus: MessageBus,
}

impl HudView {
	pub fn new(gfx: &mut gfx::System, message_bus: MessageBus) -> anyhow::Result<Self> {
		Ok(HudView {
			_message_bus: message_bus,
		})
	}

	pub fn do_ui(&mut self, ui: ui::UiContext, model: &Model) {
		let safe_area_panel = ui.begin_widget();
		safe_area_panel.layout.set_padding(8.0);

		if model.hud.in_dialog {
			self.do_dialog_ui(ui.clone(), model);
		} else {
			self.do_playing_ui(ui.clone(), model);
		}

		ui.end_widget();
	}

	fn do_playing_ui(&mut self, ui: ui::UiContext, model: &Model) {
		ui.vertical_layout(|layout| {
			layout.layout.set_padding(4.0);
			layout.layout.set_alignment(ui::Alignment::Begin, ui::Alignment::Begin);
			layout.draw_rect(Color::black().with_alpha(0.5));

			let Player { blood, salt, .. } = model.player;
			ui.text(format!("Blood: {blood}"));
			ui.text(format!("Salt: {salt}"));
		})
	}

	fn do_dialog_ui(&mut self, _ui: ui::UiContext, _model: &Model) {

	}

// 	fn draw_playing(&mut self, usable_area: Aabb2, ui_shared: &mut ui::UiShared, model: &Model) {
// 		let Player { blood, salt, .. } = model.player;

// 		let mut painter = self.painter.with_shared(ui_shared);

// 		let text_width = painter.text_rect(16, "Blood: 000").width();

// 		painter.rect(Aabb2::from_min_size(usable_area.min, Vec2::new(text_width, 32.0)).grow(4.0), Color::black().with_alpha(0.5));

// 		painter.text(usable_area.min + Vec2::from_y(0.0), 16, format!("Blood: {blood}"), Color::white());
// 		painter.text(usable_area.min + Vec2::from_y(16.0), 16, format!("Salt: {salt}"), Color::white());

// 		if let Some(hud_text) = &model.hud.hud_text {
// 			let fade_in = (hud_text.elapsed_visible_time/HUD_TEXT_FADE_IN_TIME).ease_quad_inout();
// 			let fade_out = ((HUD_TEXT_SHOW_TIME - hud_text.elapsed_visible_time)/HUD_TEXT_FADE_OUT_TIME).ease_quad_inout();
// 			let alpha = fade_in.min(fade_out).clamp(0.0, 1.0).powi(2);

// 			let text_rect = painter.text_rect(16, &hud_text.text);
// 			let text_extents = text_rect.size() / 2.0;

// 			let usable_center = usable_area.center();
// 			let text_center = usable_center - Vec2::from_y(16.0 + 16.0);
// 			let text_pos = text_center - text_extents;

// 			painter.rect(text_rect.translate(text_pos).grow(4.0), Color::black().with_alpha(0.7 * alpha));
// 			painter.text(text_pos, 16, &hud_text.text, Color::white().with_alpha(alpha));
// 		}


// 		if let Some(object) = model.interactions.hovered_object.and_then(|id| model.processed_world.objects.get(id)) {
// 			let interact_message = match &object.info {
// 				ObjectInfo::Ladder { target_world, .. } => format!("To {target_world}"),
// 				_ => format!("Frob '{}'", object.name),
// 			};

// 			let text_rect = painter.text_rect(16, &interact_message);
// 			let text_size = text_rect.size();

// 			let text_pos = usable_area.center() - text_size.to_0y()/2.0 + Vec2::from_x(14.0);
// 			// let text_pos = usable_area.center() - text_size * Vec2::new(0.5, 1.0) - Vec2::from_y(12.0);

// 			// TODO(pat.m): interaction icon
// 			painter.rect(Aabb2::from_center_extents(usable_area.center(), 8.0), Color::grey(0.5).with_alpha(0.1));

// 			painter.rect(text_rect.translate(text_pos).grow(2.0), Color::black().with_alpha(0.5));
// 			painter.text(text_pos, 16, interact_message, Color::grey(0.5));
// 		}
// 	}

// 	fn draw_dialog(&mut self, usable_area: Aabb2, ui_shared: &mut ui::UiShared, _model: &Model) {
// 		let mut painter = self.painter.with_shared(ui_shared);

// 		let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Quisque vulputate nunc orci.
// Proin varius a neque vel ultrices. Nulla facilisi. Praesent eget dictum ante.
// In blandit diam quis nibh convallis ultricies. Donec facilisis enim a mauris scelerisque,
// vitae vulputate urna mattis.";

// 		let font_size = 16;

// 		let text_bounds = painter.text_rect(font_size, text);
// 		let text_extents = text_bounds.size()/2.0;
// 		let center = usable_area.center();

// 		let bounds = Aabb2::from_center_extents(center, text_extents + Vec2::splat(16.0));

// 		painter.rect(bounds, Color::grey(0.02));
// 		painter.text(center + text_extents * Vec2::new(-1.0, 1.0) - Vec2::from_y(font_size as f32 - 4.0), font_size, text, Color::rgb(0.6, 0.1, 0.1));
// 	}
}




