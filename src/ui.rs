mod ui_painter;
mod glyph_cache;
mod layout;
mod layout_types;
mod widget_tree;

pub use ui_painter::*;
pub use layout_types::*;
pub use widget_tree::WidgetId;

use crate::prelude::*;

use glyph_cache::GlyphCache;
use widget_tree::WidgetTree;

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

pub enum UiPass<'sys> {
	// Input,
	Layout,
	Render(UiPainter<'sys>),
}

struct WidgetStackEntry {
	id: WidgetId,
	num_children: u32,
}

pub struct UiContext<'sys> {
	system: &'sys UiSystem,
	pub pass: UiPass<'sys>,

	tree: WidgetTree,
	widget_stack: Vec<WidgetStackEntry>,
}

pub fn build(ctx: &mut Context, mut do_ui: impl FnMut(&mut UiContext)) {
	let Context{ gfx, ui_system, .. } = ctx;

	let screen_size = gfx.backbuffer_size().to_vec2() * ui_system.global_scale;

	let mut ui_ctx = UiContext {
		system: ui_system,
		pass: UiPass::Layout,

		tree: WidgetTree::default(),
		widget_stack: Vec::with_capacity(16),
	};

	let root_widget = ui_ctx.tree.make_root();
	root_widget.layout.set_fixed_size(screen_size);

	// Layout pass
	{
		ui_ctx.pass = UiPass::Layout;
		ui_ctx.widget_stack.clear();
		ui_ctx.widget_stack.push(WidgetStackEntry{ id: WidgetId::ROOT, num_children: 0 });

		do_ui(&mut ui_ctx);
	}

	layout::layout_widget_tree(&mut ui_ctx.tree);

	// Render pass
	{
		let projection = Mat4::ortho(0.0, screen_size.x, 0.0, screen_size.y, -1.0, 1.0);

		let mut encoder = gfx.frame_encoder.command_group(gfx::FrameStage::Ui(0));
		encoder.bind_shared_ubo(0, &[projection]);

		let painter = UiPainter {
			buffer: UiPaintBuffer::new(),
			encoder,

			f_text_shader: ui_ctx.system.f_text_shader,
			font_atlas_image: ui_ctx.system.glyph_cache.borrow().font_atlas,

			paint_mode: UiPaintMode::ShapeUntextured,
		};

		ui_ctx.pass = UiPass::Render(painter);
		ui_ctx.widget_stack.clear();
		ui_ctx.widget_stack.push(WidgetStackEntry{ id: WidgetId::ROOT, num_children: 0 });

		do_ui(&mut ui_ctx);
	}

	let UiPass::Render(painter) = ui_ctx.pass else { panic!() };
	painter.finish();
}


pub struct WidgetRef<'sys, 'ctx> where 'sys: 'ctx {
	pub layout: &'ctx mut WidgetLayout,
	pub rect: Option<Aabb2>,

	pub pass: &'ctx mut UiPass<'sys>,
}

impl WidgetRef<'_, '_> {
	pub fn with_painter(&mut self, f: impl FnOnce(&mut UiPainter)) {
		if let UiPass::Render(painter) = self.pass {
			f(painter);
		}
	}

	pub fn with_painter_and_rect(&mut self, f: impl FnOnce(&mut UiPainter, Aabb2)) {
		if let UiPass::Render(painter) = self.pass 
			&& let Some(rect) = self.rect
		{
			f(painter, rect);
		}
	}

	pub fn draw_rect(&mut self, color: impl Into<Color>) {
		self.with_painter_and_rect(|painter, rect| {
			painter.rect(rect, color.into());
		});
	}
}


impl<'sys> UiContext<'sys> {
	pub fn begin_widget<'ctx>(&'ctx mut self) -> WidgetRef<'sys, 'ctx> {
		let id = self.auto_id();
		self.begin_widget_with_id(id)
	}

	pub fn begin_widget_with_id<'ctx>(&'ctx mut self, id: impl Into<WidgetId>) -> WidgetRef<'sys, 'ctx> {
		let id = id.into();

		let prev = self.widget_stack.last_mut().unwrap();
		let parent = prev.id;
		prev.num_children += 1;

		self.widget_stack.push(WidgetStackEntry{ id, num_children: 0 });

		let widget = match self.pass {
			UiPass::Layout => self.tree.insert(id, parent),
			UiPass::Render(_) => self.tree.get_mut(id),
		};

		WidgetRef {
			layout: &mut widget.layout,
			rect: widget.rect,

			pass: &mut self.pass,
		}
	}

	pub fn end_widget(&mut self) {
		self.widget_stack.pop();
	}

	pub fn do_widget<'ctx>(&'ctx mut self) -> WidgetRef<'sys, 'ctx> {
		let id = self.auto_id();
		self.do_widget_with_id(id)
	}

	pub fn do_widget_with_id<'ctx>(&'ctx mut self, id: impl Into<WidgetId>) -> WidgetRef<'sys, 'ctx> {
		let id = id.into();

		let prev = self.widget_stack.last_mut().unwrap();
		let parent = prev.id;
		prev.num_children += 1;

		let widget = match self.pass {
			UiPass::Layout => self.tree.insert(id, parent),
			UiPass::Render(_) => self.tree.get_mut(id),
		};

		WidgetRef {
			layout: &mut widget.layout,
			rect: widget.rect,

			pass: &mut self.pass,
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

impl<'sys> UiContext<'sys> {
	pub fn text(&mut self, text: impl AsRef<str>) {
		let mut text_layout = SmallVec::<[(Aabb2, Aabb2); 16]>::new();
		let text = text.as_ref();
		let font_size = 16;

		{
			let mut glyph_cache = self.system.glyph_cache.borrow_mut();
			glyph_cache.layout(&self.system.font, font_size, text, |glyph_geom, glyph_uvs| {
				text_layout.push((glyph_geom, glyph_uvs));
			});
		}

		let mut widget = self.do_widget();
		let text_rect = text_layout.iter().fold(Aabb2::zero(), |acc, &(rect, _)| acc.include_rect(rect));
		widget.layout.set_fixed_size(text_rect.size());

		widget.with_painter_and_rect(|painter, rect| {
			painter.set_paint_mode(UiPaintMode::Text);
			for (geom, uvs) in text_layout {
				painter.buffer.draw_quad(geom.translate(rect.min), uvs, Color::black());
			}
		});
	}

	pub fn button(&mut self, text: impl AsRef<str>) {
		self.text(text);
	}
}


