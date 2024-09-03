use crate::prelude::*;

use model::{WorldPosition, GlobalVertexId, GlobalWallId};

mod viewport;
use viewport::{Viewport, ViewportItemFlags};

mod commands;
use commands::*;

pub use commands::{handle_editor_cmds, EditorWorldEditCmd};


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Item {
	Vertex(GlobalVertexId),
	Wall(GlobalWallId),
	Room(usize),

	PlayerSpawn,
}

impl Item {
	fn room_index(&self, world: &model::World) -> usize {
		match *self {
			Item::Room(room_index) => room_index,
			Item::Vertex(GlobalVertexId{room_index, ..}) | Item::Wall(GlobalWallId{room_index, ..}) => room_index,
			Item::PlayerSpawn => world.player_spawn_position.room_index,
		}
	}

	fn set_room_index(&mut self, new_room_index: usize) {
		match self {
			Item::Room(room_index) | Item::Vertex(GlobalVertexId{room_index, ..}) | Item::Wall(GlobalWallId{room_index, ..}) => {
				*room_index = new_room_index;
			}

			_ => {}
		}
	}
}


#[derive(Debug)]
pub struct State {
	hovered: Option<Item>,
	selection: Option<Item>,

	focused_room_index: usize,

	editor_world_edit_cmd_sub: Subscription<EditorWorldEditCmd>,
}

impl State {
	pub fn new(message_bus: &MessageBus) -> Self {
		State {
			hovered: None,
			selection: None,

			focused_room_index: 0,

			editor_world_edit_cmd_sub: message_bus.subscribe(),
		}
	}
}

struct Context<'w> {
	state: &'w mut State,
	model: &'w model::Model,
	message_bus: &'w MessageBus,
}

pub fn draw_world_editor(ctx: &egui::Context, state: &mut State, model: &model::Model, message_bus: &MessageBus) {
	let mut context = Context {
		state,
		model,
		message_bus,
	};

	egui::Window::new("World Settings")
		.show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.label("Fog Color");

				let mut fog_color = model.world.fog_color;
				if ui.color_edit_button_rgb(fog_color.as_mut()).changed() {
					message_bus.emit(EditorWorldEditCmd::SetFogParams(fog_color));
				}
			});

			ui.horizontal(|ui| {
				ui.label("Spawn");

				let WorldPosition{ room_index, local_position: Vec2{x, y} } = model.world.player_spawn_position;
				let yaw = model.world.player_spawn_yaw;

				ui.label(format!("Room #{room_index} <{x:.1}, {y:.1}>, {:.1}°", yaw.to_degrees()));

				if ui.button("Set Here").clicked() {
					message_bus.emit(EditorWorldEditCmd::SetPlayerSpawn);
				}
			});

			// TODO(pat.m): player spawn location
		});

	egui::Window::new("All Rooms")
		.show(ctx, |ui| {
			// ui.with_layout(egui::Layout::right_to_left(egui::Align::Center) , |ui| {
			// 	ui.menu_button("...", |ui| {
			// 		if ui.button("Center Player").clicked() {
			// 			// context.state.focused_room_index = model.player.position.room_index;
			// 			// TODO(pat.m): some viewport shit
			// 			ui.close_menu();
			// 		}
			// 	});
			// });

			draw_all_room_viewport(ui, &mut context);
		});

	egui::Window::new("Focused Room")
		.show(ctx, |ui| {
			ui.horizontal(|ui| {
				draw_room_selector(ui, &mut context);

				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center) , |ui| {
					ui.menu_button("...", |ui| {
						if ui.button("Focus Player").clicked() {
							context.state.selection = Some(Item::Room(model.player.position.room_index));
							// TODO(pat.m): recenter viewport
							ui.close_menu();
						}
					});
				});
			});

			draw_focused_room_viewport(ui, &mut context);
		});

	egui::Window::new("Inspector")
		.show(ctx, |ui| {
			draw_item_inspector(ui, &mut context);
		});

	// egui::Window::new("Internal State")
	// 	.show(ctx, |ui| {
	// 		ui.label(format!("{state:#?}"));
	// 	});
}


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
		if ui.add(egui::widgets::Slider::new(&mut height, 0.1..=5.0).step_by(0.01).logarithmic(true))
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

fn draw_wall_inspector(ui: &mut egui::Ui, Context{model, message_bus, ..}: &mut Context, wall_id: GlobalWallId) {
	let GlobalWallId{room_index, wall_index} = wall_id;

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
		if ui.add(egui::widgets::Slider::new(&mut offset, -2.0..=2.0).step_by(0.01).clamp_to_range(false))
			.changed()
		{
			message_bus.emit(EditorWorldEditCmd::SetHorizontalWallOffset(wall_id, offset));
		}
	});

	ui.horizontal(|ui| {
		ui.label("Vertical Offset");
		
		let mut offset = wall.vertical_offset;
		if ui.add(egui::widgets::Slider::new(&mut offset, -1.0..=1.0).step_by(0.01).clamp_to_range(false))
			.changed()
		{
			message_bus.emit(EditorWorldEditCmd::SetVerticalWallOffset(wall_id, offset));
		}
	});
}




const PLAYER_SPAWN_COLOR: Color = Color::rgb(1.0, 0.3, 0.1);


fn draw_focused_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let focused_room_index = context.state.selection.as_ref()
		.map_or(context.state.focused_room_index, |item| item.room_index(&context.model.world));

	let neighbouring_room_margin = 0.3;

	let mut neighbouring_rooms = Vec::new();

	for wall_index in 0..context.model.world.rooms[focused_room_index].walls.len() {
		let src_wall_id = GlobalWallId{room_index: focused_room_index, wall_index};
		let Some(tgt_wall_id) = context.model.world.wall_target(src_wall_id)
			else { continue };

		let (start, end) = context.model.world.wall_vertices(src_wall_id);
		let wall_normal = (end - start).normalize().perp();

		let transform = model::calculate_portal_transform(&context.model.world, src_wall_id, tgt_wall_id);
		let offset_transform = Mat2x3::translate(wall_normal * neighbouring_room_margin) * transform;

		neighbouring_rooms.push((tgt_wall_id.room_index, offset_transform));
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

	viewport.add_player_indicator(world.player_spawn_position, world.player_spawn_yaw, Item::PlayerSpawn, PLAYER_SPAWN_COLOR, ViewportItemFlags::empty());
	viewport.add_player_indicator(player.position, player.yaw, None, Color::grey(0.8), ViewportItemFlags::empty());

	viewport.build()
}



fn draw_all_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let world = &context.model.world;
	let player = &context.model.player;

	let mut viewport = Viewport::new(ui, context);
	let mut position = Vec2::zero();
	let mut max_height = 0.0f32;

	let margin = 1.0;
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

	viewport.add_player_indicator(world.player_spawn_position, world.player_spawn_yaw, Item::PlayerSpawn, PLAYER_SPAWN_COLOR, ViewportItemFlags::empty());
	viewport.add_player_indicator(player.position, player.yaw, None, Color::grey(0.8), ViewportItemFlags::empty());

	viewport.build()
}


