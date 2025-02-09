use crate::prelude::*;

use model::{SourceModel, Placement, ObjectId, VertexId, WallId, RoomId};

mod viewport;
use viewport::{Viewport, ViewportItemFlags};

mod commands;
pub use commands::*;

mod undo;
use undo::*;

mod world_editor;
mod inspector;

use world_editor::do_world_editor;
use inspector::do_inspector;

use std::borrow::Cow;
use egui::widgets::Slider;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Item {
	Vertex(VertexId),
	Wall(WallId),
	Room(RoomId),

	PlayerSpawn,
	Object(ObjectId),
}

impl Item {
	fn room_id(&self, world: &model::World) -> RoomId {
		let geometry = &world.geometry;

		match *self {
			Item::Room(room_id) => room_id,
			Item::Vertex(vertex_id) => {
				let wall_id = geometry.vertices[vertex_id].outgoing_wall;
				geometry.walls[wall_id].room
			}
			Item::Wall(wall_id) => geometry.walls[wall_id].room,
			Item::PlayerSpawn => world.player_spawn.room_id,
			Item::Object(object_id) => world.objects.get(object_id)
				.map(|obj| obj.placement.room_id)
				.unwrap(),
		}
	}
}


#[derive(Debug)]
pub struct State {
	inner: InnerState,
	pub undo_stack: UndoStack,

	editor_world_edit_cmd_sub: Subscription<EditorWorldEditCmd>,
	undo_cmd_sub: Subscription<UndoCmd>,
	// editor_modal_cmd_sub: Subscription<EditorModalCmd>,
}

#[derive(Debug)]
struct InnerState {
	hovered: Option<Item>,
	selection: Option<Item>,

	focused_room_id: Option<RoomId>,
	track_player: bool,
}

impl State {
	pub fn new(message_bus: &MessageBus) -> Self {
		State {
			inner: InnerState {
				hovered: None,
				selection: None,

				focused_room_id: None,
				track_player: true,
			},

			undo_stack: UndoStack::new(message_bus.clone()),
			editor_world_edit_cmd_sub: message_bus.subscribe(),
			undo_cmd_sub: message_bus.subscribe(),
			// editor_modal_cmd_sub: message_bus.subscribe(),
		}
	}

	pub fn reset(&mut self) {
		self.undo_stack.clear();
		self.inner.focused_room_id = None;
		self.inner.hovered = None;
		self.inner.selection = None;
	}
}

struct Context<'w> {
	state: &'w mut InnerState,
	model: &'w model::SourceModel,
	runtime_model: &'w model::Model,
	message_bus: &'w MessageBus,

	source_player_placement: Placement,
}

fn validate_model(state: &mut State, model: &mut SourceModel) {
	let geometry = &model.world.geometry;
	let first_room = geometry.first_room();

	if !state.inner.focused_room_id.map_or(false, |room_id| room_id.is_valid(geometry)) {
		state.inner.focused_room_id = Some(first_room);
	}

	// Yuck
	if !model.world.player_spawn.room_id.is_valid(geometry) {
		model.world.player_spawn.room_id = first_room;
	}

	validate_item(model, &mut state.inner.hovered);
	validate_item(model, &mut state.inner.selection);
}


fn validate_item(model: &SourceModel, maybe_item: &mut Option<Item>) {
	let geometry = &model.world.geometry;
	let objects = &model.world.objects;

	match maybe_item {
		&mut Some(Item::Object(object_id)) if !objects.contains_key(object_id) => {
			*maybe_item = None;
		}

		&mut Some(Item::Wall(wall_id)) if !wall_id.is_valid(geometry) => {
			*maybe_item = None;
		}

		&mut Some(Item::Vertex(vertex_id)) if !vertex_id.is_valid(geometry) => {
			*maybe_item = None;
		}

		Some(item) if !item.room_id(&model.world).is_valid(geometry) => {
			*maybe_item = None;
		}

		_ => {}
	}
}



pub fn do_editor(ui_ctx: &egui::Context, state: &mut State, model: &model::SourceModel, runtime_model: &model::Model, message_bus: &MessageBus) {
	// TODO(pat.m): modal world load/save flows
	let modal_active = false;

	if !modal_active {
		let undo_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z);
		let redo_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z);

		ui_ctx.input_mut(|input| {
			if input.consume_shortcut(&redo_shortcut) {
				message_bus.emit(UndoCmd::Redo);
			}

			if input.consume_shortcut(&undo_shortcut) {
				message_bus.emit(UndoCmd::Undo);
			}
		});
	}

	if state.inner.focused_room_id.is_none() {
		let first_room = model.world.geometry.first_room();
		state.inner.focused_room_id = Some(first_room);
	}

	egui::Window::new("Undo Stack")
		.enabled(!modal_active)
		.show(ui_ctx, |ui| do_undo_stack_widget(ui, &state.undo_stack, message_bus));

	let mut context = Context {
		state: &mut state.inner,
		model,
		runtime_model,
		message_bus,

		source_player_placement: runtime_model.processed_world.to_source_placement(runtime_model.player.placement),
	};

	egui::SidePanel::right("Inspector")
		.show(ui_ctx, |ui| {
			ui.add_enabled_ui(!modal_active, |ui| {
				do_inspector(ui, &mut context);
			});
		});

	// TODO(pat.m): toggle
	// egui::Window::new("Geometry")

	let frame = egui::Frame::dark_canvas(&ui_ctx.style()).multiply_with_opacity(0.9);
	egui::CentralPanel::default()
		.frame(frame)
		.show(ui_ctx, |ui| do_world_editor(ui, &mut context));
}

pub fn do_undo_stack_widget(ui: &mut egui::Ui, undo_stack: &UndoStack, message_bus: &MessageBus) {
	ui.horizontal(|ui| {
		if ui.button("Undo").clicked() {
			message_bus.emit(UndoCmd::Undo);
		}

		ui.label(format!("{} / {}", undo_stack.index(), undo_stack.len()));

		if ui.button("Redo").clicked() {
			message_bus.emit(UndoCmd::Redo);
		}
	});

	let text_style = egui::TextStyle::Body;
	let row_height = ui.text_style_height(&text_style);

	egui::ScrollArea::vertical()
		.show_rows(ui, row_height, undo_stack.len(), |ui, range| {
			if range.start == 0 {
				let active = undo_stack.index() == 0;

				if ui.selectable_label(active, "<base>").clicked() {
					message_bus.emit(UndoCmd::SetIndex(0));
				}
			}

			for index in range {
				let active = undo_stack.index() == index+1;
				if ui.selectable_label(active, undo_stack.describe(index)).clicked() {
					message_bus.emit(UndoCmd::SetIndex(index+1));
				}
			}
		});

	ui.collapsing("Debug", |ui| {
		ui.label(format!("{:#?}", undo_stack));
	});
}