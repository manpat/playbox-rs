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


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UiPass {
	// Input,
	Layout,
	Render,
}


struct UiContextImpl {
	system: *mut UiSystem,
	input: *mut input::System,
	pass: UiPass,

	ref_count: usize,
	rw_lock: i32,
}

impl UiContextImpl {
	fn assert_unused(&self) {
		assert!(self.ref_count == 0);
		assert!(self.rw_lock == 0);
	}
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

	pub fn with_ui_system_mut<R: 'static>(&self, f: impl FnOnce(&mut UiSystem) -> R) -> R {
		self.write(move |ctx| unsafe {
			f(ctx.system.as_mut().unwrap())
		})
	}

	pub fn with_ui_system<R: 'static>(&self, f: impl FnOnce(&UiSystem) -> R) -> R {
		self.read(move |ctx| unsafe {
			f(ctx.system.as_ref().unwrap())
		})
	}

	pub fn with_input_system<R: 'static>(&self, f: impl FnOnce(&input::System) -> R) -> R {
		self.read(move |ctx| unsafe {
			f(ctx.input.as_ref().unwrap())
		})
	}

	pub fn with_widget_tree_mut<R: 'static>(&self, f: impl FnOnce(&mut WidgetTree) -> R) -> R {
		self.with_ui_system_mut(move |system| {
			f(&mut system.widget_tree)
		})
	}

	pub fn with_painter(&self, f: impl FnOnce(&mut UiPainter)) {
		if self.read(|ctx| ctx.pass != UiPass::Render) {
			return
		}

		self.with_ui_system_mut(move |system| {
			f(&mut system.painter);
		});
	}
}


pub fn build(ctx: &mut Context, mut do_ui: impl FnMut(UiContext)) {
	let Context{ gfx, input, ui_system, .. } = ctx;

	let screen_size = gfx.backbuffer_size().to_vec2() * ui_system.global_scale;

	ui_system.widget_tree.reset();

	let root_widget = ui_system.widget_tree.get_mut(WidgetId::ROOT);
	unsafe {
		(*root_widget.layout).set_fixed_size(screen_size);
	}

	// Layout pass
	{
		let mut ui_ctx_impl = UiContextImpl {
			system: *ui_system,
			input: *input,
			pass: UiPass::Layout,

			ref_count: 0,
			rw_lock: 0,
		};

		do_ui(UiContext::new(&mut ui_ctx_impl));

		ui_ctx_impl.assert_unused();
	}

	layout::layout_widget_tree(&mut ui_system.widget_tree);

	ui_system.widget_tree.reset();

	// Render pass
	{
		let mut ui_ctx_impl = UiContextImpl {
			system: *ui_system,
			input: *input,
			pass: UiPass::Render,

			ref_count: 0,
			rw_lock: 0,
		};

		do_ui(UiContext::new(&mut ui_ctx_impl));

		ui_ctx_impl.assert_unused();
	}

	ui_system.painter.finish(gfx, &ui_system.text_rendering, screen_size);
}


pub struct WidgetRef<'ctx> {
	pub ctx: UiContext,
	pub id: WidgetId,

	pub layout: &'ctx mut WidgetLayout,
	pub rect: Aabb2,
}

impl WidgetRef<'_> {
	pub fn with_painter(&self, f: impl FnOnce(&mut UiPainter)) {
		self.ctx.with_painter(f);
	}

	pub fn draw_rect(&self, color: impl Into<Color>) {
		self.ctx.with_painter(|painter| {
			painter.fill_solid_quad(self.rect, color.into());
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

		let (rect, layout) = self.with_widget_tree_mut(|widget_tree| {
			let widget = widget_tree.track_widget_start(id);
			(widget.rect, widget.layout)
		});

		WidgetRef {
			ctx: self.clone(),
			id,

			layout: unsafe { &mut *layout },
			rect,
		}
	}

	pub fn end_widget(&self) {
		self.with_widget_tree_mut(|widget_tree| widget_tree.track_widget_end());
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

		self.with_ui_system(|system| {
			let current_widget = system.widget_tree.submission_stack.last().unwrap();
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

		self.with_ui_system_mut(|system| {
			system.text_rendering.glyph_cache.layout(&system.text_rendering.font, font_size, text, |glyph_geom, glyph_uvs| {
				text_layout.push((glyph_geom, glyph_uvs));
			});
		});

		let widget = self.do_widget();

		let text_rect = text_layout.iter().fold(Aabb2::zero(), |acc, &(rect, _)| acc.include_rect(rect));
		widget.layout.set_fixed_size(text_rect.size());

		self.with_painter(|painter| {
			painter.set_paint_mode(UiPaintMode::Text);
			for (geom, uvs) in text_layout {
				painter.add_quad(geom.translate(widget.rect.min), uvs, Color::black());
			}
		});
	}

	pub fn button(&self, text: impl AsRef<str>) -> bool {
		let button = self.begin_widget();
		button.layout.set_type(LayoutType::LeftToRight);
		button.layout.set_child_alignment(Alignment::Center, Alignment::Center);
		button.layout.set_padding(4.0);
		button.layout.vertical.set_fixed_size(16.0 + 8.0);
		button.draw_rect(Color::magenta());

		let ui_scale = self.with_ui_system(|system| system.global_scale);

		// TODO(pat.m): hotness should be calculated ahead of time so occlusion can be taken into account
		// + so we can handle stuff like dragging
		let (is_hot, is_clicked) = self.with_input_system(|input| {
			let Some(position) = input.mouse_position_pixels() else { return (false, false) };
			let is_hot = button.rect.contains_point(position * ui_scale);
			(is_hot, is_hot && input.button_just_up(input::MouseButton::Left))
		});

		self.text(text);

		// Hover state
		if is_hot {
			button.draw_rect(Color::grey_a(1.0, 0.12));
		}

		self.end_widget();

		is_clicked
	}
}


