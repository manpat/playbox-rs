mod ui_painter;
mod glyph_cache;
mod layout;
mod system;
mod layout_types;
mod widget_tree;

pub use ui_painter::*;
pub use system::UiSystem;
pub use layout_types::*;
pub use widget_tree::WidgetId;

use crate::prelude::*;

use glyph_cache::GlyphCache;
use widget_tree::WidgetTree;


pub enum UiPass {
	// Input,
	Layout,
	Render(UiPainter),
}

struct WidgetStackEntry {
	id: WidgetId,
	num_children: u32,
}

struct UiContextImpl {
	system: *mut UiSystem,
	pass: UiPass,

	tree: WidgetTree,
	widget_stack: Vec<WidgetStackEntry>,

	ref_count: usize,
	rw_lock: i32,
}

pub struct UiContext(*mut UiContextImpl);

impl Clone for UiContext {
	fn clone(&self) -> Self {
		unsafe{ *self.ref_count_mut() += 1; }
		Self(self.0)
	}
}

impl Drop for UiContext {
	fn drop(&mut self) {
		let rc = self.ref_count_mut();
		unsafe {
			match rc.read().checked_sub(1) {
				Some(value) => { rc.write(value) }
				None => panic!("UiContext refcount underflow"),
			}
		}
	}
}

impl UiContext {
	fn new(ctx_impl: &mut UiContextImpl) -> UiContext {
		let ctx = UiContext(ctx_impl);
		unsafe {
			*ctx.ref_count_mut() += 1;
		}
		ctx
	}

	fn ref_count_mut(&self) -> *mut usize {
		unsafe {
			&raw mut (*self.0).ref_count
		}
	}
	fn rw_lock_mut(&self) -> *mut i32 {
		unsafe {
			&raw mut (*self.0).rw_lock
		}
	}

	fn read<R: 'static>(&self, f: impl FnOnce(&UiContextImpl) -> R) -> R {
		let rw_lock = self.rw_lock_mut();
		let current = unsafe {
			let current = rw_lock.read();
			assert!(current >= 0);
			rw_lock.write(current + 1);
			current
		};

		let result = f(unsafe{ &*self.0 });

		unsafe {
			assert!(rw_lock.read() == current + 1);
			rw_lock.write(current);
		}

		result
	}

	fn write<R: 'static>(&self, f: impl FnOnce(&mut UiContextImpl) -> R) -> R {
		let rw_lock = self.rw_lock_mut();
		unsafe {
			assert!(rw_lock.read() == 0);
			rw_lock.write(-1);
		}

		let result = f(unsafe{ &mut *self.0 });

		unsafe {
			assert!(rw_lock.read() == -1);
			rw_lock.write(0);
		}

		result
	}

	pub fn with_painter(&self, f: impl FnOnce(&mut UiPainter)) {
		self.write(move |ctx| {
			if let UiPass::Render(painter) = &mut ctx.pass {
				f(painter);
			}
		});
	}

	pub fn with_system(&self, f: impl FnOnce(&mut UiSystem)) {
		// TODO(pat.m): NOT SAFE if we ever independently borrow parts of UiSystem
		self.write(move |ctx| unsafe {
			f(ctx.system.as_mut().unwrap());
		});
	}
}


pub fn build(ctx: &mut Context, mut do_ui: impl FnMut(UiContext)) {
	let Context{ gfx, ui_system, .. } = ctx;

	let screen_size = gfx.backbuffer_size().to_vec2() * ui_system.global_scale;

	let mut ui_ctx_impl = UiContextImpl {
		system: *ui_system,
		pass: UiPass::Layout,

		tree: WidgetTree::new(),
		widget_stack: Vec::with_capacity(16),

		ref_count: 0,
		rw_lock: 0,
	};


	let root_widget = ui_ctx_impl.tree.make_root();
	unsafe {
		(*root_widget.layout).set_fixed_size(screen_size);
	}

	// Layout pass
	{
		ui_ctx_impl.pass = UiPass::Layout;
		ui_ctx_impl.widget_stack.clear();
		ui_ctx_impl.widget_stack.push(WidgetStackEntry{ id: WidgetId::ROOT, num_children: 0 });

		do_ui(UiContext::new(&mut ui_ctx_impl));
	}

	layout::layout_widget_tree(&mut ui_ctx_impl.tree);

	// Render pass
	{
		let projection = Mat4::ortho(0.0, screen_size.x, 0.0, screen_size.y, -1.0, 1.0);

		let mut encoder = gfx.frame_encoder.command_group(gfx::FrameStage::Ui(0));
		encoder.bind_shared_ubo(0, &[projection]);

		let painter = UiPainter {
			buffer: UiPaintBuffer::new(),
			// encoder,

			// TODO(pat.m): THIS IS NOT ACTUALLY SOUND
			// since mutable references to UiSystem will be made during `do_ui`, they are technically aliasing.
			// But once I move UiPainter _into_ UiSystem this will no longer be necessary + these will never
			// actually change so _is_ safe.
			f_text_shader: ui_system.f_text_shader,
			font_atlas_image: ui_system.glyph_cache.font_atlas,

			paint_mode: UiPaintMode::ShapeUntextured,
		};

		ui_ctx_impl.pass = UiPass::Render(painter);
		ui_ctx_impl.widget_stack.clear();
		ui_ctx_impl.widget_stack.push(WidgetStackEntry{ id: WidgetId::ROOT, num_children: 0 });

		do_ui(UiContext::new(&mut ui_ctx_impl));
	}

	assert!(ui_ctx_impl.ref_count == 0);
	assert!(ui_ctx_impl.rw_lock == 0);

	let UiPass::Render(painter) = ui_ctx_impl.pass else { panic!() };
	painter.finish();
}


pub struct WidgetRef<'ctx> {
	pub ctx: UiContext,

	pub layout: &'ctx mut WidgetLayout,
	pub rect: Option<Aabb2>,
}

impl WidgetRef<'_> {
	pub fn with_painter(&self, f: impl FnOnce(&mut UiPainter)) {
		self.ctx.with_painter(f);
	}

	pub fn with_painter_and_rect(&self, f: impl FnOnce(&mut UiPainter, Aabb2)) {
		if let Some(rect) = self.rect {
			self.ctx.with_painter(|painter| f(painter, rect));
		}
	}

	pub fn draw_rect(&self, color: impl Into<Color>) {
		self.with_painter_and_rect(|painter, rect| {
			painter.rect(rect, color.into());
		});
	}
}


impl UiContext {
	pub fn begin_widget(&self) -> WidgetRef {
		let id = self.auto_id();
		self.begin_widget_with_id(id)
	}

	pub fn begin_widget_with_id(&self, id: impl Into<WidgetId>) -> WidgetRef {
		let id = id.into();

		let (rect, layout) = self.write(|ctx| {
			let prev = ctx.widget_stack.last_mut().unwrap();
			let parent = prev.id;
			prev.num_children += 1;

			ctx.widget_stack.push(WidgetStackEntry{ id, num_children: 0 });

			let widget = match ctx.pass {
				UiPass::Layout => ctx.tree.insert(id, parent),
				UiPass::Render(_) => ctx.tree.get_mut(id),
			};

			(widget.rect, widget.layout)
		});

		WidgetRef {
			ctx: self.clone(),

			layout: unsafe { &mut *layout },
			rect,
		}
	}

	pub fn end_widget(&self) {
		self.write(|ctx| ctx.widget_stack.pop());
	}

	pub fn do_widget(&self) -> WidgetRef {
		let id = self.auto_id();
		self.do_widget_with_id(id)
	}

	pub fn do_widget_with_id(&self, id: impl Into<WidgetId>) -> WidgetRef {
		let widget_ref = self.begin_widget_with_id(id);
		self.end_widget();
		widget_ref
	}

	pub fn auto_id(&self) -> WidgetId {
		let hasher = &mut DefaultHasher::new();

		self.read(|ctx| {
			let current_widget = ctx.widget_stack.last().unwrap();
			current_widget.id.hash(hasher);
			current_widget.num_children.hash(hasher);
		});

		WidgetId(hasher.finish())
	}
}

impl UiContext {
	pub fn text(&self, text: impl AsRef<str>) {
		let mut text_layout = SmallVec::<[(Aabb2, Aabb2); 16]>::new();
		let text = text.as_ref();
		let font_size = 16;

		self.with_system(|system| {
			system.glyph_cache.layout(&system.font, font_size, text, |glyph_geom, glyph_uvs| {
				text_layout.push((glyph_geom, glyph_uvs));
			});
		});

		let widget = self.do_widget();
		let text_rect = text_layout.iter().fold(Aabb2::zero(), |acc, &(rect, _)| acc.include_rect(rect));
		widget.layout.set_fixed_size(text_rect.size());

		widget.with_painter_and_rect(|painter, rect| {
			painter.set_paint_mode(UiPaintMode::Text);
			for (geom, uvs) in text_layout {
				painter.buffer.draw_quad(geom.translate(rect.min), uvs, Color::black());
			}
		});
	}

	pub fn button(&self, text: impl AsRef<str>) {
		self.text(text);
	}
}


