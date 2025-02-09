
use crate::editor::*;
use model::*;

pub fn do_inspector(ui: &mut egui::Ui, ctx: &mut Context) {
	ui.heading("World Settings");
	draw_world_settings(ui, ctx);

	ui.separator();

	ui.heading("Objects");
	draw_object_list(ui, ctx);

	ui.separator();

	ui.heading("Inspector");
	draw_item_inspector(ui, ctx);
}

fn draw_world_settings(ui: &mut egui::Ui, ctx: &mut Context) {
	ui.horizontal(|ui| {
		ui.label("Player Spawn");

		let Placement{ room_id, position: Vec2{x, y}, yaw } = ctx.model.world.player_spawn;
		ui.label(format!("{room_id:?} <{x:.1}, {y:.1}>, {:.1}°", yaw.to_degrees()));

		if ui.button("Set Here").clicked() {
			ctx.message_bus.emit(EditorWorldEditCmd::SetPlayerSpawn(ctx.source_player_placement));
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
				placement: ctx.source_player_placement,
				info: model::ObjectInfo::Debug,
			};

			ctx.message_bus.emit(EditorWorldEditCmd::AddObject(object));
		}

		if ui.button("Ladder").clicked() {
			let object = model::Object {
				name: "ladder".to_string(),
				placement: ctx.source_player_placement,
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
				placement: ctx.source_player_placement,
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
			for (object_id, object) in ctx.model.world.objects.iter() {
				let is_selected = ctx.state.selection == Some(Item::Object(object_id));
				let response = ui.selectable_label(is_selected, match object.name.is_empty() {
					true => "<no name>",
					false => object.name.as_str(),
				});

				if response.clicked() {
					ctx.state.selection = Some(Item::Object(object_id));
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

		// Some(Item::Vertex(vertex_id)) => {
		// 	draw_room_inspector(ui, ctx, vertex_id.room_index);
		// }

		Some(Item::Wall(wall_id)) => {
			// draw_room_inspector(ui, ctx, wall_id.room_index);
			// ui.separator();
			draw_wall_inspector(ui, ctx, wall_id);
		}

		Some(Item::Room(room_id)) => {
			draw_room_inspector(ui, ctx, room_id);
		}

		Some(Item::Object(object_id)) => {
			draw_object_inspector(ui, ctx, object_id);
		}

		_ => {
			ui.label("<unimplemented>");
		}
	}

}

fn draw_room_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, room_id: RoomId) {
	let Some(room) = model.world.geometry.rooms.get(room_id) else {
		return
	};

	ui.label(format!("Room #{room_id:?}"));

	ui.horizontal(|ui| {
		ui.label("Ceiling Color");

		let mut ceiling_color = room.ceiling_color;
		if ui.color_edit_button_rgb(ceiling_color.as_mut()).changed() {
			message_bus.emit(EditorWorldEditCmd::SetCeilingColor(room_id, ceiling_color));
		}
	});

	ui.horizontal(|ui| {
		ui.label("Ceiling Height");

		let mut height = room.height;
		if ui.add(Slider::new(&mut height, 0.1..=5.0).logarithmic(true))
			.changed()
		{
			message_bus.emit(EditorWorldEditCmd::SetCeilingHeight(room_id, height));
		}
	});

	ui.separator();

	ui.horizontal(|ui| {
		ui.label("Floor Color");

		let mut floor_color = room.floor_color;
		if ui.color_edit_button_rgb(floor_color.as_mut()).changed() {
			message_bus.emit(EditorWorldEditCmd::SetFloorColor(room_id, floor_color));
		}
	});
}

fn draw_wall_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, wall_id: WallId) {
	let Some(wall) = model.world.geometry.walls.get(wall_id) else {
		return
	};

	ui.label(format!("Wall #{wall_id:?}"));

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
		if ui.add(Slider::new(&mut offset, -2.0..=2.0).step_by(0.01).clamp_to_range(false))
			.changed()
		{
			message_bus.emit(EditorWorldEditCmd::SetVerticalWallOffset(wall_id, offset));
		}
	});
}

fn draw_object_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, object_id: ObjectId) {
	let Some(object) = model.world.objects.get(object_id) else {
		return
	};

	ui.label(format!("{object_id:?} - \"{}\"", object.name));

	ui.horizontal(|ui| {
		ui.label("Name");

		let mut object_name = Cow::from(&object.name);
		if ui.text_edit_singleline(&mut object_name).changed() {
			message_bus.emit(EditorWorldEditCmd::SetObjectName(object_id, object_name.into_owned()));
		}
	});

	match &object.info {
		ObjectInfo::Ladder{target_world, target_object} => {
			ui.separator();

			let mut target_world = Cow::from(target_world);
			if ui.text_edit_singleline(&mut target_world).changed() {
				let new_target_world = target_world.into_owned();
				message_bus.emit(EditorWorldEditCmd::edit_object(object_id, move |_, object| {
					if let ObjectInfo::Ladder{target_world, ..} = &mut object.info {
						*target_world = new_target_world;
					}
				}));
			}

			let mut target_object = Cow::from(target_object);
			if ui.text_edit_singleline(&mut target_object).changed() {
				let new_target_object = target_object.into_owned();
				message_bus.emit(EditorWorldEditCmd::edit_object(object_id, move |_, object| {
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
					message_bus.emit(EditorWorldEditCmd::edit_object(object_id, move |_, object| {
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
					message_bus.emit(EditorWorldEditCmd::edit_object(object_id, move |_, object| {
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
					message_bus.emit(EditorWorldEditCmd::edit_object(object_id, move |_, object| {
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
					message_bus.emit(EditorWorldEditCmd::edit_object(object_id, move |_, object| {
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
