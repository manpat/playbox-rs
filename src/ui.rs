mod ui_painter;
// mod ui_builder;
mod ui_layout;
mod glyph_cache;

pub use ui_painter::*;
// pub use ui_builder::*;
pub use ui_layout::*;

use crate::prelude::*;
use glyph_cache::GlyphCache;
use std::cell::RefCell;

// const FONT_DATA: &[u8] = include_bytes!("../resource/fonts/Tuffy.otf");
// const FONT_DATA: &[u8] = include_bytes!("../resource/fonts/Quicksand-Light.ttf");
const FONT_DATA: &[u8] = include_bytes!("../resource/fonts/Saga 8.ttf");
// const FONT_DATA: &[u8] = include_bytes!("../resource/fonts/Outflank 9.ttf");

pub struct UiSystem {
	// Font rendering
	pub font: fontdue::Font,
	pub glyph_cache: RefCell<GlyphCache>,

	f_text_shader: gfx::ShaderHandle,

	// Storage
	// TODO(pat.m): ...

	// Config
	pub global_scale: f32,
}

impl UiSystem {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<UiSystem> {
		let font = fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
			.map_err(|err| anyhow::anyhow!("{err}"))?;

		Ok(UiSystem {
			font,
			glyph_cache: RefCell::new(GlyphCache::new(gfx)),

			f_text_shader: gfx.resource_manager.load_fragment_shader("shaders/text.fs.glsl"),

			global_scale: 0.5,
		})
	}

	pub fn update(&mut self, gfx: &mut gfx::System) {
		self.glyph_cache.get_mut().update_atlas(gfx);
	}
}

pub enum UiPass<'ctx> {
	// Input,
	Layout,
	Render(UiPainter<'ctx>),
}

struct WidgetStackEntry {
	id: WidgetId,
	num_children: u32,
}

pub struct UiContext<'ctx> {
	system: &'ctx mut UiSystem,
	pass: UiPass<'ctx>,

	tree: WidgetTree,
	widget_stack: Vec<WidgetStackEntry>,
}

pub fn build_ui(gfx: &mut gfx::System, system: &mut UiSystem, mut do_ui: impl FnMut(&mut UiContext)) {
	let screen_size = gfx.backbuffer_size().to_vec2() * system.global_scale;

	let mut ctx = UiContext {
		system,
		pass: UiPass::Layout,

		tree: WidgetTree::default(),
		widget_stack: Vec::with_capacity(16),
	};

	let root_widget = ctx.tree.make_root();
	root_widget.constraints.set_fixed_size(screen_size);

	// Layout pass
	{
		ctx.pass = UiPass::Layout;
		ctx.widget_stack.clear();
		ctx.widget_stack.push(WidgetStackEntry{ id: WidgetId::ROOT, num_children: 0 });

		do_ui(&mut ctx);
	}

	// layout_widget_tree(ctx.tree);

	// Render pass
	{
		let projection = Mat4::ortho(0.0, screen_size.x, 0.0, screen_size.y, -1.0, 1.0);

		let mut encoder = gfx.frame_encoder.command_group(gfx::FrameStage::Ui(0));
		encoder.bind_shared_ubo(0, &[projection]);

		let painter = UiPainter {
			buffer: UiPaintBuffer::new(),
			encoder,

			f_text_shader: ctx.system.f_text_shader,
			font_atlas_image: ctx.system.glyph_cache.borrow().font_atlas,

			paint_mode: UiPaintMode::ShapeUntextured,
		};

		ctx.pass = UiPass::Render(painter);
		ctx.widget_stack.clear();
		ctx.widget_stack.push(WidgetStackEntry{ id: WidgetId::ROOT, num_children: 0 });

		do_ui(&mut ctx);
	}

	let UiPass::Render(painter) = ctx.pass else { panic!() };
	painter.finish();
}



impl<'ctx> UiContext<'ctx> {
	pub fn start_widget(&mut self, id: impl Into<WidgetId>) -> &mut Widget {
		let id = id.into();

		let prev = self.widget_stack.last_mut().unwrap();
		let parent = prev.id;
		prev.num_children += 1;

		self.widget_stack.push(WidgetStackEntry{ id, num_children: 0 });

		match self.pass {
			UiPass::Layout => self.tree.insert(id, parent),
			UiPass::Render(_) => self.tree.get_mut(id),
		}
	}

	pub fn end_widget(&mut self) {
		self.widget_stack.pop();
	}

	pub fn do_widget(&mut self, id: impl Into<WidgetId>) -> &mut Widget {
		let id = id.into();

		let prev = self.widget_stack.last_mut().unwrap();
		let parent = prev.id;
		prev.num_children += 1;

		match self.pass {
			UiPass::Layout => self.tree.insert(id, parent),
			UiPass::Render(_) => self.tree.get_mut(id),
		}
	}

	pub fn auto_id(&mut self) -> WidgetId {
		let hasher = &mut DefaultHasher::new();
		let current_widget = self.widget_stack.last().unwrap();

		current_widget.id.hash(hasher);
		current_widget.num_children.hash(hasher);

		WidgetId(hasher.finish())
	}
}

impl<'ctx> UiContext<'ctx> {
	pub fn text(&mut self, text: impl AsRef<str>) {
		let id = self.auto_id();
		let widget = self.do_widget(id);

		let text = text.as_ref();
		let font_size = 16;

		match self.pass {
			UiPass::Layout => {
				// let text_rect = self.system.painter.text_rect(font_size, text);
				// widget.constraints.set_fixed_size(text_rect.size());
			}

			UiPass::Render(_) => {
				// self.system.painter.text(font_size, text);
			}

			_ => {}
		}
	}

	pub fn button(&mut self, text: impl AsRef<str>) {
		self.text(text);
	}
}




#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
enum Axis {
	Horizontal = 0,
	Vertical = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
enum LayoutType {
	Stack,
	LeftToRight,
	RightToLeft,
	TopToBottom,
	BottomToTop,
}

struct WidgetAxisConstraints {
	pub min: f32,
	pub preferred: f32,
	pub max: f32,

	pub padding_start: f32,
	pub padding_end: f32,

	pub margin_start: f32,
	pub margin_end: f32,
}

impl Default for WidgetAxisConstraints {
	fn default() -> Self {
		WidgetAxisConstraints {
			min: 0.0,
			preferred: 1.0,
			max: f32::INFINITY,

			padding_start: 0.0,
			padding_end: 0.0,

			margin_start: 0.0,
			margin_end: 0.0,
		}
	}
}

impl WidgetAxisConstraints {
	pub fn set_fixed_size(&mut self, fixed: f32) {
		self.min = fixed;
		self.preferred = fixed;
		self.max = fixed;
	}
}

#[derive(Default)]
struct WidgetConstraints([WidgetAxisConstraints; 2]);

impl WidgetConstraints {
	pub fn axis(&self, axis: Axis) -> &WidgetAxisConstraints {
		&self.0[axis as usize]
	}

	pub fn axis_mut(&mut self, axis: Axis) -> &mut WidgetAxisConstraints {
		&mut self.0[axis as usize]
	}

	pub fn set_fixed_size(&mut self, size: Vec2) {
		self.axis_mut(Axis::Horizontal).set_fixed_size(size.x);
		self.axis_mut(Axis::Vertical).set_fixed_size(size.y);
	}
}

struct Widget {
	pub constraints: WidgetConstraints,
	pub parent: WidgetId,

	// Includes padding.
	pub rect: Option<Aabb2>,
}

impl Default for Widget {
	fn default() -> Widget {
		Widget {
			constraints: default(),
			parent: WidgetId::ROOT,

			rect: None,
		}
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct WidgetId(u64);

impl WidgetId {
	pub const ROOT: WidgetId = WidgetId(0);
}


#[derive(Default)]
struct WidgetTree {
	widgets: HashMap<WidgetId, Widget>,
	children: HashMap<WidgetId, SmallVec<[WidgetId; 4]>>,
}

impl WidgetTree {
	pub fn clear(&mut self) {
		self.widgets.clear();
		self.children.clear();
	}

	pub fn make_root(&mut self) -> &mut Widget {
		use std::collections::hash_map::Entry;

		match self.widgets.entry(WidgetId::ROOT) {
			Entry::Occupied(_) => panic!("Root already exists!"),
			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent: WidgetId::ROOT,
					.. default()
				})
			}
		}
	}

	pub fn get_mut(&mut self, id: WidgetId) -> &mut Widget {
		self.widgets.get_mut(&id).expect("Requesting widget not submitted in layout pass")
	}

	pub fn insert(&mut self, id: WidgetId, parent: WidgetId) -> &mut Widget {
		use std::collections::hash_map::Entry;

		assert!(id != WidgetId::ROOT);

		self.children.entry(parent)
			.or_default()
			.push(id);

		match self.widgets.entry(id) {
			Entry::Occupied(_) => panic!("Conflicting id! {id:?}"),
			Entry::Vacant(entry) => {
				entry.insert(Widget {
					parent,
					.. default()
				})
			}
		}
	}
}