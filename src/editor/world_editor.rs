// use crate::prelude::*;
use crate::editor::{Context, *};
use model::*;


#[instrument(skip_all, name="editor draw_world_editor")]
pub fn draw_world_editor(ctx: &egui::Context, state: &mut State, model: &model::Model, message_bus: &MessageBus) {
	// TODO(pat.m): modal world load/save flows
	let modal_active = false;

	if !modal_active {
		let undo_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z);
		let redo_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z);

		ctx.input_mut(|input| {
			if input.consume_shortcut(&redo_shortcut) {
				message_bus.emit(UndoCmd::Redo);
			}

			if input.consume_shortcut(&undo_shortcut) {
				message_bus.emit(UndoCmd::Undo);
			}
		});
	}

	let mut context = Context {
		state: &mut state.inner,
		model,
		message_bus,
	};

	egui::Window::new("All Rooms")
		.enabled(!modal_active)
		.show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.checkbox(&mut context.state.track_player, "Track Player");
			});

			draw_all_room_viewport(ui, &mut context);
		});

	egui::Window::new("Focused Room")
		.enabled(!modal_active)
		.show(ctx, |ui| {
			ui.horizontal(|ui| {
				draw_room_selector(ui, &mut context);

				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center) , |ui| {
					ui.menu_button("...", |ui| {
						if ui.button("Focus Player").clicked() {
							context.state.selection = Some(Item::Room(model.player.placement.room_index));
							// TODO(pat.m): recenter viewport
							ui.close_menu();
						}
					});
				});
			});

			draw_focused_room_viewport(ui, &mut context);
		});

	egui::SidePanel::right("Inspector")
		.show(ctx, |ui| {
			ui.add_enabled_ui(!modal_active, |ui| {
				ui.heading("World Settings");
				draw_world_settings(ui, &mut context);

				ui.separator();

				ui.heading("Objects");
				draw_object_list(ui, &mut context);

				ui.separator();

				ui.heading("Inspector");
				draw_item_inspector(ui, &mut context);
			});
		});

	egui::Window::new("Undo Stack")
		.enabled(!modal_active)
		.show(ctx, |ui| {
			ui.horizontal(|ui| {
				if ui.button("Undo").clicked() {
					context.message_bus.emit(UndoCmd::Undo);
				}

				ui.label(format!("{} / {}", state.undo_stack.index(), state.undo_stack.len()));

				if ui.button("Redo").clicked() {
					context.message_bus.emit(UndoCmd::Redo);
				}
			});

			let text_style = egui::TextStyle::Body;
			let row_height = ui.text_style_height(&text_style);

			egui::ScrollArea::vertical()
				.show_rows(ui, row_height, state.undo_stack.len(), |ui, range| {
					if range.start == 0 {
						let active = state.undo_stack.index() == 0;

						if ui.selectable_label(active, "<base>").clicked() {
							context.message_bus.emit(UndoCmd::SetIndex(0));
						}
					}

					for index in range {
						let active = state.undo_stack.index() == index+1;
						if ui.selectable_label(active, state.undo_stack.describe(index)).clicked() {
							context.message_bus.emit(UndoCmd::SetIndex(index+1));
						}
					}
				});

			ui.collapsing("Debug", |ui| {
				ui.label(format!("{:#?}", state.undo_stack));
			});
		});
}

fn draw_world_settings(ui: &mut egui::Ui, ctx: &mut Context) {
	ui.horizontal(|ui| {
		ui.label("Player Spawn");

		let Placement{ room_index, position: Vec2{x, y}, yaw } = ctx.model.world.player_spawn;
		ui.label(format!("Room #{room_index} <{x:.1}, {y:.1}>, {:.1}°", yaw.to_degrees()));

		if ui.button("Set Here").clicked() {
			ctx.message_bus.emit(EditorWorldEditCmd::SetPlayerSpawn);
		}
	});

	let mut fog = ctx.model.world.fog;
	let mut changed = false;

	egui::Grid::new("world_settings").show(ui, |ui| {
		ui.label("Fog Color");
		changed |= ui.color_edit_button_rgb(fog.color.as_mut()).changed();

		ui.end_row();

		ui.label("Fog Start");
		changed |= slider_widget(ui, &mut fog.start, 0.0..=5.0);
		ui.end_row();

		ui.label("Fog Distance");
		changed |= log_slider_widget(ui, &mut fog.distance, 1.0..=300.0);
		ui.end_row();

		ui.label("Fog Emission");
		let mut emission_non_linear = (fog.emission * 2.0 + 1.0).ln();
		changed |= slider_widget(ui, &mut emission_non_linear, 0.0..=1.0);
		fog.emission = (emission_non_linear.exp() - 1.0) / 2.0;
		ui.end_row();

		ui.label("Fog Transparency");
		changed |= slider_widget(ui, &mut fog.transparency, 0.0..=1.0);
		ui.end_row();
	});

	if changed {
		ctx.message_bus.emit(EditorWorldEditCmd::SetFogParams(fog));
	}
	
	use egui::emath::Numeric;

	fn slider_widget<N: Numeric>(ui: &mut egui::Ui, value: &mut N, range: std::ops::RangeInclusive<N>) -> bool {
		ui.add(Slider::new(value, range)).changed()
	}

	fn log_slider_widget<N: Numeric>(ui: &mut egui::Ui, value: &mut N, range: std::ops::RangeInclusive<N>) -> bool {
		ui.add(Slider::new(value, range).logarithmic(true)).changed()
	}
}


fn draw_object_list(ui: &mut egui::Ui, ctx: &mut Context) {
	ui.horizontal(|ui| {
		if ui.button("Debug").clicked() {
			let object = model::Object {
				name: "Debug Object".to_string(),
				placement: ctx.model.player.placement,
				info: model::ObjectInfo::Debug,
			};

			ctx.message_bus.emit(EditorWorldEditCmd::AddObject(object));
		}

		if ui.button("Ladder").clicked() {
			let object = model::Object {
				name: "ladder".to_string(),
				placement: ctx.model.player.placement,
				info: model::ObjectInfo::Ladder {
					target_world: "world2".into(),
					target_object: "ladder".into(),
				},
			};

			ctx.message_bus.emit(EditorWorldEditCmd::AddObject(object));
		}

		if ui.button("Light").clicked() {
			let object = model::Object {
				name: "light".to_string(),
				placement: ctx.model.player.placement,
				info: model::ObjectInfo::Light(LightObject{
					color: Color::white(),
					height: 0.5,
					power: 1.0,
					radius: 1.0,
				}),
			};

			ctx.message_bus.emit(EditorWorldEditCmd::AddObject(object));
		}
	});

	egui::ScrollArea::vertical()
		.show(ui, |ui| {
			for (object_index, object) in ctx.model.world.objects.iter().enumerate() {
				let is_selected = ctx.state.selection == Some(Item::Object(object_index));
				let response = ui.selectable_label(is_selected, match object.name.is_empty() {
					true => "<no name>",
					false => object.name.as_str(),
				});

				if response.clicked() {
					ctx.state.selection = Some(Item::Object(object_index));
				}
			}
		});
}

fn draw_item_inspector(ui: &mut egui::Ui, ctx: &mut Context) {
	ui.spacing_mut().slider_width = 200.0;

	match ctx.state.selection {
		None => {
			ui.label("<select an item>");
		}

		Some(Item::Vertex(vertex_id)) => {
			draw_room_inspector(ui, ctx, vertex_id.room_index);
		}

		Some(Item::Wall(wall_id)) => {
			draw_room_inspector(ui, ctx, wall_id.room_index);
			ui.separator();
			draw_wall_inspector(ui, ctx, wall_id);
		}

		Some(Item::Room(room_index)) => {
			draw_room_inspector(ui, ctx, room_index);
		}

		Some(Item::Object(object_index)) => {
			draw_object_inspector(ui, ctx, object_index);
		}

		_ => {
			ui.label("<unimplemented>");
		}
	}

}

fn draw_room_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, room_index: usize) {
	let Some(room) = model.world.rooms.get(room_index) else {
		return
	};

	ui.label(format!("Room #{room_index}"));

	ui.horizontal(|ui| {
		ui.label("Ceiling Color");

		let mut ceiling_color = room.ceiling_color;
		if ui.color_edit_button_rgb(ceiling_color.as_mut()).changed() {
			message_bus.emit(EditorWorldEditCmd::SetCeilingColor(room_index, ceiling_color));
		}
	});

	ui.horizontal(|ui| {
		ui.label("Ceiling Height");

		let mut height = room.height;
		if ui.add(Slider::new(&mut height, 0.1..=5.0).step_by(0.01).logarithmic(true))
			.changed()
		{
			message_bus.emit(EditorWorldEditCmd::SetCeilingHeight(room_index, height));
		}
	});

	ui.separator();

	ui.horizontal(|ui| {
		ui.label("Floor Color");

		let mut floor_color = room.floor_color;
		if ui.color_edit_button_rgb(floor_color.as_mut()).changed() {
			message_bus.emit(EditorWorldEditCmd::SetFloorColor(room_index, floor_color));
		}
	});
}

fn draw_wall_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, wall_id: WallId) {
	let WallId{room_index, wall_index} = wall_id;

	let Some(wall) = model.world.rooms.get(room_index)
		.and_then(|room| room.walls.get(wall_index))
	else {
		return
	};

	ui.label(format!("Wall #{wall_index}"));

	ui.horizontal(|ui| {
		ui.label("Color");

		let mut wall_color = wall.color;
		if ui.color_edit_button_rgb(wall_color.as_mut()).changed() {
			message_bus.emit(EditorWorldEditCmd::SetWallColor(wall_id, wall_color));
		}
	});

	ui.horizontal(|ui| {
		ui.label("horizontal Offset");

		let mut offset = wall.horizontal_offset;
		if ui.add(Slider::new(&mut offset, -2.0..=2.0).step_by(0.01).clamp_to_range(false))
			.changed()
		{
			message_bus.emit(EditorWorldEditCmd::SetHorizontalWallOffset(wall_id, offset));
		}
	});

	ui.horizontal(|ui| {
		ui.label("Vertical Offset");

		let mut offset = wall.vertical_offset;
		if ui.add(Slider::new(&mut offset, -1.0..=1.0).step_by(0.01).clamp_to_range(false))
			.changed()
		{
			message_bus.emit(EditorWorldEditCmd::SetVerticalWallOffset(wall_id, offset));
		}
	});
}

fn draw_object_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, object_index: usize) {
	let Some(object) = model.world.objects.get(object_index) else {
		return
	};

	ui.label(format!("Object #{object_index} - \"{}\"", object.name));

	ui.horizontal(|ui| {
		ui.label("Name");

		let mut object_name = Cow::from(&object.name);
		if ui.text_edit_singleline(&mut object_name).changed() {
			message_bus.emit(EditorWorldEditCmd::SetObjectName(object_index, object_name.into_owned()));
		}
	});

	match &object.info {
		ObjectInfo::Ladder{target_world, target_object} => {
			ui.separator();

			let mut target_world = Cow::from(target_world);
			if ui.text_edit_singleline(&mut target_world).changed() {
				let new_target_world = target_world.into_owned();
				message_bus.emit(EditorWorldEditCmd::edit_object(object_index, move |_, object| {
					if let ObjectInfo::Ladder{target_world, ..} = &mut object.info {
						*target_world = new_target_world;
					}
				}));
			}

			let mut target_object = Cow::from(target_object);
			if ui.text_edit_singleline(&mut target_object).changed() {
				let new_target_object = target_object.into_owned();
				message_bus.emit(EditorWorldEditCmd::edit_object(object_index, move |_, object| {
					if let ObjectInfo::Ladder{target_object, ..} = &mut object.info {
						*target_object = new_target_object;
					}
				}));
			}
		}

		&ObjectInfo::Light(LightObject{color, height, power, radius}) => {
			ui.separator();

			ui.horizontal(|ui| {
				ui.label("Color");

				let mut light_color = color;
				if ui.color_edit_button_rgb(light_color.as_mut()).changed() {
					message_bus.emit(EditorWorldEditCmd::edit_object(object_index, move |_, object| {
						if let Some(LightObject{color, ..}) = object.as_light_mut() {
							*color = light_color;
						}
					}));
				}
			});

			ui.horizontal(|ui| {
				ui.label("Height");

				let mut new_height = height;
				if ui.add(Slider::new(&mut new_height, 0.0..=5.0).step_by(0.01)).changed() {
					message_bus.emit(EditorWorldEditCmd::edit_object(object_index, move |_, object| {
						if let Some(LightObject{height, ..}) = object.as_light_mut() {
							*height = new_height;
						}
					}));
				}
			});

			ui.horizontal(|ui| {
				ui.label("Radius");

				let mut new_radius = radius;
				if ui.add(Slider::new(&mut new_radius, 0.1..=20.0).logarithmic(false)).changed() {
					message_bus.emit(EditorWorldEditCmd::edit_object(object_index, move |_, object| {
						if let Some(LightObject{radius, ..}) = object.as_light_mut() {
							*radius = new_radius;
						}
					}));
				}
			});

			ui.horizontal(|ui| {
				ui.label("Power");

				let mut new_power = power;
				if ui.add(Slider::new(&mut new_power, 0.1..=100.0).step_by(0.01).logarithmic(true)).changed() {
					message_bus.emit(EditorWorldEditCmd::edit_object(object_index, move |_, object| {
						if let Some(LightObject{power, ..}) = object.as_light_mut() {
							*power = new_power;
						}
					}));
				}
			});
		}

		_ => {}
	}
}




const PLAYER_SPAWN_COLOR: Color = Color::rgb(1.0, 0.3, 0.1);
const OBJECT_COLOR: Color = Color::rgb(0.3, 0.8, 0.5);


fn draw_room_selector(ui: &mut egui::Ui, Context{model, state, ..}: &mut Context) {
	let selected_room_index = state.selection.as_ref().map_or(state.focused_room_index, |item| item.room_index(&model.world));

	ui.horizontal(|ui| {
		for (room_index, _room) in model.world.rooms.iter().enumerate() {
			let selected = room_index == selected_room_index;
			if ui.selectable_label(selected, format!("{room_index}")).clicked() {
				state.selection = Some(Item::Room(room_index));
			}
		}
	});
}


fn draw_focused_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let focused_room_index = match (context.state.track_player, &context.state.selection) {
		(true, _) => context.model.player.placement.room_index,
		(false, Some(item)) => item.room_index(&context.model.world),
		(false, None) => context.state.focused_room_index,
	};

	let neighbouring_room_margin = 0.3;

	let mut neighbouring_rooms = Vec::new();

	for wall_index in 0..context.model.world.rooms[focused_room_index].walls.len() {
		let src_wall_id = WallId{room_index: focused_room_index, wall_index};
		if let Some(wall_info) = context.model.processed_world.wall_info(src_wall_id)
			&& let Some(connection_info) = &wall_info.connection_info
		{
			let offset_transform = Mat2x3::translate(wall_info.normal * neighbouring_room_margin) * connection_info.target_to_source;

			neighbouring_rooms.push((connection_info.target_id.room_index, offset_transform));
		}
	}

	let player = &context.model.player;
	let world = &context.model.world;

	let mut viewport = Viewport::new(ui, context);
	viewport.add_room(focused_room_index, Mat2x3::identity(), ViewportItemFlags::BASIC_INTERACTIONS | ViewportItemFlags::RECENTERABLE);
	viewport.add_room_connections(focused_room_index, Mat2x3::identity(), ViewportItemFlags::BASIC_INTERACTIONS);

	for (room_index, transform) in neighbouring_rooms {
		viewport.add_room(room_index, transform, ViewportItemFlags::BASIC_INTERACTIONS);
		viewport.add_room_connections(room_index, transform, ViewportItemFlags::empty());
	}

	for (object_index, object) in world.objects.iter().enumerate() {
		viewport.add_object(object.placement, Item::Object(object_index), OBJECT_COLOR, ViewportItemFlags::BASIC_INTERACTIONS);
	}

	viewport.add_player_indicator(world.player_spawn, Item::PlayerSpawn, PLAYER_SPAWN_COLOR, ViewportItemFlags::empty());
	viewport.add_player_indicator(player.placement, None, Color::grey(0.8), ViewportItemFlags::empty());

	viewport.build()
}



fn draw_all_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let world = &context.model.world;
	let player = &context.model.player;

	let mut viewport = Viewport::new(ui, context);
	let mut position = Vec2::zero();
	let mut max_height = 0.0f32;

	let margin = 0.4;
	let per_row = 5;

	for (room_index, room) in world.rooms.iter().enumerate() {
		let bounds = room.bounds();
		let room_size = bounds.size();
		let offset = position + room_size / 2.0 - bounds.center();

		viewport.add_room(room_index, Mat2x3::translate(offset), ViewportItemFlags::BASIC_INTERACTIONS);
		viewport.add_room_connections(room_index, Mat2x3::translate(offset), ViewportItemFlags::BASIC_INTERACTIONS);

		max_height = max_height.max(room_size.y);

		position.x += room_size.x + margin;
		if room_index % per_row == per_row - 1 {
			position.x = 0.0;
			position.y += max_height + margin;
			max_height = 0.0;
		}
	}

	for (object_index, object) in world.objects.iter().enumerate() {
		viewport.add_object(object.placement, Item::Object(object_index), OBJECT_COLOR, ViewportItemFlags::BASIC_INTERACTIONS);
	}

	viewport.add_player_indicator(world.player_spawn, Item::PlayerSpawn, PLAYER_SPAWN_COLOR, ViewportItemFlags::empty());
	viewport.add_player_indicator(player.placement, None, Color::grey(0.8), ViewportItemFlags::empty());

	viewport.build()
}