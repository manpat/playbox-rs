use crate::prelude::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Axis {
	Horizontal,
	Vertical,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Alignment {
	Begin,
	Center,
	// TODO(pat.m): baseline
	End,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum LayoutType {
	#[default]
	Stack,
	LeftToRight,
	RightToLeft,
	TopToBottom,
	BottomToTop,
}

#[derive(Clone)]
pub struct WidgetAxisLayout {
	pub min: f32,
	pub preferred: f32, // TODO(pat.m): do I actually need this?
	pub max: f32,

	pub padding_start: f32,
	pub padding_end: f32,

	pub margin_start: f32,
	pub margin_end: f32,

	pub spacing: f32,

	pub child_alignment: Alignment,
	pub alignment: Option<Alignment>,
}

bitflags! {
	#[derive(Copy, Clone, Debug)]
	pub struct WidgetLayoutFlags : u32 {

	}
}

impl Default for WidgetAxisLayout {
	fn default() -> Self {
		WidgetAxisLayout {
			min: 0.0,
			preferred: 1.0,
			max: f32::INFINITY,

			padding_start: 0.0,
			padding_end: 0.0,

			margin_start: 0.0,
			margin_end: 0.0,

			spacing: 4.0,

			child_alignment: Alignment::Center,
			alignment: None,
		}
	}
}

impl WidgetAxisLayout {
	pub fn set_fixed_size(&mut self, fixed: f32) {
		self.min = fixed;
		self.preferred = fixed;
		self.max = fixed;
	}

	pub fn set_margins(&mut self, size: f32) {
		self.margin_start = size;
		self.margin_end = size;
	}

	pub fn set_paddings(&mut self, size: f32) {
		self.padding_start = size;
		self.padding_end = size;
	}
}

#[derive(Default, Clone)]
pub struct WidgetLayout {
	pub layout_type: LayoutType,
	pub horizontal: WidgetAxisLayout,
	pub vertical: WidgetAxisLayout,
}

impl WidgetLayout {
	pub fn axis(&self, axis: Axis) -> &WidgetAxisLayout {
		match axis {
			Axis::Horizontal => &self.horizontal,
			Axis::Vertical => &self.vertical,
		}
	}

	pub fn axis_mut(&mut self, axis: Axis) -> &mut WidgetAxisLayout {
		match axis {
			Axis::Horizontal => &mut self.horizontal,
			Axis::Vertical => &mut self.vertical,
		}
	}

	pub fn each<T>(&mut self, f: impl Fn(&WidgetAxisLayout) -> T) -> [T; 2] {
		[f(&self.horizontal), f(&self.vertical)]
	}

	pub fn set_fixed_size(&mut self, size: Vec2) {
		self.horizontal.set_fixed_size(size.x);
		self.vertical.set_fixed_size(size.y);
	}

	pub fn set_margins(&mut self, margin: f32) {
		self.horizontal.set_margins(margin);
		self.vertical.set_margins(margin);
	}

	pub fn set_paddings(&mut self, padding: f32) {
		self.horizontal.set_paddings(padding);
		self.vertical.set_paddings(padding);
	}
}

