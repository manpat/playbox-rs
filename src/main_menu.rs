use crate::prelude::*;



pub fn do_main_menu_ui(ctx: &mut Context<'_>) {
	ctx.gfx.frame_encoder.backbuffer_color(Color::rgb(0.05, 0.01, 0.01));

	ctx.input.set_capture_mouse(false);

	ui::build(ctx, |ui| {
		let bus = ui.message_bus();

		let widget = ui.begin_widget();
		widget.layout.set_type(ui::LayoutType::LeftToRight);
		widget.layout.set_margin(10.0);
		widget.layout.set_padding(10.0);
		widget.layout.set_child_alignment(ui::Alignment::Begin, ui::Alignment::Center);

		{
			let widget = ui.begin_widget();
			widget.layout.set_type(ui::LayoutType::TopToBottom);
			widget.layout.fit_to_contents();
			widget.layout.vertical.set_child_alignment(ui::Alignment::Center);
			widget.layout.set_padding(10.0);
			widget.draw_rect(Color::grey_a(0.0, 0.3));

			let space_pressed = ui.with_input_system(|input| input.button_just_down(input::keys::Space));

			if ui.button("Play") || space_pressed {
				// ctx.audio.trigger();
				bus.emit(MenuCmd::Play("default".into()));
			}

			ui.button("Settings");

			if ui.button("Quit") {
				bus.emit(MenuCmd::QuitToDesktop);
			}

			ui.end_widget();
		}

		{
			ui.do_widget();
		}

		ui.end_widget();
	});
}


pub enum MenuCmd {
	Play(String),
	PlayGeneratedWorld,
	Pause,
	Resume,
	Settings,
	QuitToMain,
	QuitToDesktop,
}




pub fn do_pause_menu_ui(ctx: &mut Context) {
	if ctx.input.button_just_down(input::keys::Escape) {
		ctx.bus.emit(MenuCmd::Resume);
	}

	ctx.input.set_capture_mouse(false);

	ui::build(ctx, |ui| {
		let bus = ui.message_bus();

		let widget = ui.begin_widget();
		widget.layout.set_type(ui::LayoutType::TopToBottom);
		widget.layout.fit_to_contents();
		widget.layout.vertical.set_child_alignment(ui::Alignment::Center);
		widget.layout.set_padding(10.0);
		widget.draw_rect(Color::grey_a(0.0, 0.3));

		if ui.button("Resume") {
			bus.emit(MenuCmd::Resume);
		}

		if ui.button("Quit To Menu") {
			bus.emit(MenuCmd::QuitToMain);
		}

		if ui.button("Quit To Desktop") {
			bus.emit(MenuCmd::QuitToDesktop);
		}

		ui.end_widget();
	});
}




pub struct PauseMenuScene {
	// painter: ui::UiPainter,
}

impl PauseMenuScene {
	pub fn new(ctx: &mut Context<'_>) -> anyhow::Result<PauseMenuScene> {
		Ok(PauseMenuScene{
			// painter: ui::UiPainter::new(&mut ctx.gfx, MAIN_MENU_FRAME_STAGE),
		})
	}

	pub fn update(&mut self, ctx: &mut Context<'_>) {
		// ctx.input.set_capture_mouse(false);

		// let scale_factor = 0.5;

		// let size = ctx.gfx.backbuffer_size().to_vec2() * scale_factor;
		// let screen_rect = Aabb2::new(Vec2::zero(), size);
		// let mut content_rect = screen_rect.shrink(8.0); // pad edge

		// // Cap size to 200px x 200px
		// {
		// 	let Vec2{x, y} = content_rect.size() - Vec2::new(200.0, 200.0);
		// 	content_rect = content_rect.shrink(Vec2::new(x.max(0.0)/2.0, y.max(0.0)/2.0));
		// }

		// content_rect = content_rect.floor();

		// self.painter.rect(content_rect, Color::grey_a(0.0, 0.8));

		// let mut builder = self.painter.builder(ctx, ui::DumbLayout::new(content_rect.shrink(8.0)));
		// builder.input_scale_factor = scale_factor;

		// if builder.button("Resume") || ctx.input.button_just_down(input::keys::Escape) {
		// 	ctx.bus.emit(MenuCmd::Resume);
		// }

		// if builder.button("Quit To Menu") {
		// 	ctx.bus.emit(MenuCmd::QuitToMain);
		// }

		// if builder.button("Quit To Desktop") {
		// 	ctx.bus.emit(MenuCmd::QuitToDesktop);
		// }

		// self.painter.submit(&mut ctx.gfx, ctx.ui_shared, screen_rect);
	}
}