// use crate::prelude::*;
use crate::editor::{Context, *};
// use model::*;



#[instrument(skip_all, name="editor do_world_editor")]
pub fn do_world_editor(ui: &mut egui::Ui, ctx: &mut Context) {
	ui.horizontal(|ui| {
		draw_room_selector(ui, ctx);

		ui.with_layout(egui::Layout::right_to_left(egui::Align::Center) , |ui| {
			ui.menu_button("...", |ui| {
				if ui.button("Focus Player").clicked() {
					ctx.state.selection = Some(Item::Room(ctx.source_player_placement.room_id));
					// TODO(pat.m): recenter viewport
					ui.close_menu();
				}
				if ui.checkbox(&mut ctx.state.track_player, "Track Player").changed() {
					ui.close_menu();
				}
			});
		});
	});

	draw_focused_room_viewport(ui, ctx);
}



const PLAYER_SPAWN_COLOR: Color = Color::rgb(1.0, 0.3, 0.1);
const OBJECT_COLOR: Color = Color::rgb(0.3, 0.8, 0.5);


fn draw_room_selector(ui: &mut egui::Ui, Context{model, state, ..}: &mut Context) {
	let selected_room_id = state.selection.as_ref().map(|item| item.room_id(&model.world)).or(state.focused_room_id);

	ui.horizontal(|ui| {
		for room_id in model.world.geometry.rooms.keys() {
			let selected = Some(room_id) == selected_room_id;
			if ui.selectable_label(selected, format!("{room_id:?}")).clicked() {
				state.selection = Some(Item::Room(room_id));
			}
		}
	});
}


fn draw_focused_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let focused_room_id = match (context.state.track_player, &context.state.selection, context.state.focused_room_id) {
		(true, _, _) => context.source_player_placement.room_id,
		(false, Some(item), _) => item.room_id(&context.model.world),
		(false, None, Some(focused_room_id)) => focused_room_id,
		_ => todo!()
	};

	let neighbouring_room_margin = 0.3;

	let mut neighbouring_rooms = Vec::new();

	for src_wall_id in context.model.world.geometry.room_walls(focused_room_id) {
		if let Some(wall_info) = context.runtime_model.processed_world.wall_info(src_wall_id)
			&& let Some(connection_info) = &wall_info.connection_info
		{
			let offset_transform = Mat2x3::translate(wall_info.normal * neighbouring_room_margin) * connection_info.target_to_source;

			let source_room_id = context.runtime_model.processed_world.to_source_room(connection_info.target_room);
			neighbouring_rooms.push((source_room_id, offset_transform));
		}
	}

	let world = &context.model.world;
	let source_player_placement = context.source_player_placement;

	let mut viewport = Viewport::new(ui, context);
	viewport.add_room(focused_room_id, Mat2x3::identity(), ViewportItemFlags::BASIC_INTERACTIONS | ViewportItemFlags::RECENTERABLE);
	viewport.add_room_connections(focused_room_id, Mat2x3::identity(), ViewportItemFlags::BASIC_INTERACTIONS);

	for (room_id, transform) in neighbouring_rooms {
		viewport.add_room(room_id, transform, ViewportItemFlags::BASIC_INTERACTIONS);
		viewport.add_room_connections(room_id, transform, ViewportItemFlags::empty());
	}

	for (object_id, object) in world.objects.iter() {
		viewport.add_object(object.placement, Item::Object(object_id), OBJECT_COLOR, ViewportItemFlags::BASIC_INTERACTIONS);
	}

	viewport.add_player_indicator(world.player_spawn, Item::PlayerSpawn, PLAYER_SPAWN_COLOR, ViewportItemFlags::empty());
	viewport.add_player_indicator(source_player_placement, None, Color::grey(0.8), ViewportItemFlags::empty());

	viewport.build()
}



fn draw_all_room_viewport(ui: &mut egui::Ui, context: &mut Context) -> egui::Response {
	let world = &context.model.world;
	let source_player_placement = context.source_player_placement;

	let mut viewport = Viewport::new(ui, context);
	let mut position = Vec2::zero();
	let mut max_height = 0.0f32;

	let margin = 0.4;
	let per_row = 5;

	for (room_index, room_id) in world.geometry.rooms.keys().enumerate() {
		let bounds = world.geometry.room_bounds(room_id);
		let room_size = bounds.size();
		let offset = position + room_size / 2.0 - bounds.center();

		viewport.add_room(room_id, Mat2x3::translate(offset), ViewportItemFlags::BASIC_INTERACTIONS);
		viewport.add_room_connections(room_id, Mat2x3::translate(offset), ViewportItemFlags::BASIC_INTERACTIONS);

		max_height = max_height.max(room_size.y);

		position.x += room_size.x + margin;
		if room_index % per_row == per_row - 1 {
			position.x = 0.0;
			position.y += max_height + margin;
			max_height = 0.0;
		}
	}

	for (object_id, object) in world.objects.iter() {
		viewport.add_object(object.placement, Item::Object(object_id), OBJECT_COLOR, ViewportItemFlags::BASIC_INTERACTIONS);
	}

	viewport.add_player_indicator(world.player_spawn, Item::PlayerSpawn, PLAYER_SPAWN_COLOR, ViewportItemFlags::empty());
	viewport.add_player_indicator(source_player_placement, None, Color::grey(0.8), ViewportItemFlags::empty());

	viewport.build()
}