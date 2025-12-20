use crate::prelude::*;


pub enum MenuCmd {
	Play(String),
	PlayGeneratedWorld,
	Pause,
	Resume,
	Settings,
	QuitToMain,
	QuitToDesktop,
}




pub fn do_main_menu_ui(ctx: &mut Context<'_>) {
	ctx.gfx.frame.set_backbuffer_color(Color::rgb(0.05, 0.01, 0.01));

	ctx.input.set_capture_mouse(false);

	ui::build(ctx, |ui| {
		let bus = ui.message_bus();

		ui.vertical_layout(|widget| {
			widget.with_layout(|layout| {
				layout.horizontal.set_alignment(ui::Alignment::Begin);
				layout.vertical.set_child_alignment(ui::Alignment::Center);
				layout.set_margin(8.0);
				layout.set_padding(8.0);
			});

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
		});
	});
}



pub fn do_pause_menu_ui(ctx: &mut Context) {
	if ctx.input.button_just_down(input::keys::Escape) {
		ctx.bus.emit(MenuCmd::Resume);
	}

	ctx.input.set_capture_mouse(false);

	ui::build(ctx, |ui| {
		let bus = ui.message_bus();

		ui.vertical_layout(|widget| {
			widget.with_layout(|layout| layout.set_padding(10.0));
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
		});
	});
}

