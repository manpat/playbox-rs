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
	pub preferred: f32,
	pub max: f32,

	pub padding: AxisLengths,
	pub margin: AxisLengths,

	pub spacing: f32,

	pub child_alignment: Alignment,
	pub alignment: Option<Alignment>,

	pub flags: WidgetLayoutFlags,
}

bitflags! {
	#[derive(Copy, Clone, Debug)]
	pub struct WidgetLayoutFlags : u32 {
		// Instead of expanding to fill all available space, constrain size to
		// the preferred size of contents.
		const FIT_TO_CONTENTS = 1 << 0;

		// TODO(pat.m): ...
		// // Allow children to expand beyond the size of the container.
		// // E.g., scrollable areas.
		// // Children won't grow beyond their preferred sizes.
		// const OVERFLOW = 1 << 1;
	}
}

impl Default for WidgetAxisLayout {
	fn default() -> Self {
		WidgetAxisLayout {
			min: 0.0,
			preferred: 1.0,
			max: f32::INFINITY,

			padding: default(),
			margin: default(),
			spacing: 4.0,

			child_alignment: Alignment::Center,
			alignment: None,

			flags: WidgetLayoutFlags::empty(),
		}
	}
}

impl WidgetAxisLayout {
	pub fn set_fixed_size(&mut self, fixed: f32) {
		self.min = fixed;
		self.preferred = fixed;
		self.max = fixed;
	}

	pub fn set_margin(&mut self, lengths: impl Into<AxisLengths>) {
		self.margin = lengths.into();
	}

	pub fn set_padding(&mut self, lengths: impl Into<AxisLengths>) {
		self.padding = lengths.into();
	}

	pub fn set_alignment(&mut self, alignment: Alignment) {
		self.alignment = Some(alignment);
	}

	pub fn set_child_alignment(&mut self, alignment: Alignment) {
		self.child_alignment = alignment;
	}

	pub fn fit_to_contents(&mut self) {
		self.flags.insert(WidgetLayoutFlags::FIT_TO_CONTENTS);
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

	pub fn set_type(&mut self, ty: LayoutType) {
		self.layout_type = ty;
	}

	pub fn fit_to_contents(&mut self) {
		self.horizontal.fit_to_contents();
		self.vertical.fit_to_contents();
	}

	pub fn set_fixed_size(&mut self, size: Vec2) {
		self.horizontal.set_fixed_size(size.x);
		self.vertical.set_fixed_size(size.y);
	}

	pub fn set_margin(&mut self, margin: impl Into<BoxLengths>) {
		let margin = margin.into();
		self.horizontal.set_margin(margin.horizontal);
		self.vertical.set_margin(margin.vertical);
	}

	pub fn set_padding(&mut self, padding: impl Into<BoxLengths>) {
		let padding = padding.into();
		self.horizontal.set_padding(padding.horizontal);
		self.vertical.set_padding(padding.vertical);
	}

	pub fn set_alignment(&mut self, horizontal: Alignment, vertical: Alignment) {
		self.horizontal.set_alignment(horizontal);
		self.vertical.set_alignment(vertical);
	}

	pub fn set_child_alignment(&mut self, horizontal: Alignment, vertical: Alignment) {
		self.horizontal.set_child_alignment(horizontal);
		self.vertical.set_child_alignment(vertical);
	}
}



#[derive(Copy, Clone, Debug)]
pub struct AxisLengths {
	pub start: f32,
	pub end: f32,
}

impl AxisLengths {
	pub fn total(&self) -> f32 {
		self.start + self.end
	}
}

impl Default for AxisLengths {
	fn default() -> AxisLengths {
		AxisLengths { start: 0.0, end: 0.0 }
	}
}

impl From<f32> for AxisLengths {
	fn from(o: f32) -> AxisLengths {
		AxisLengths {
			start: o,
			end: o,
		}
	}
}

impl From<(f32, f32)> for AxisLengths {
	fn from((start, end): (f32, f32)) -> AxisLengths {
		AxisLengths { start, end }
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub struct BoxLengths {
	pub horizontal: AxisLengths,
	pub vertical: AxisLengths,
}

impl From<f32> for BoxLengths {
	fn from(o: f32) -> BoxLengths {
		BoxLengths {
			horizontal: AxisLengths::from(o),
			vertical: AxisLengths::from(o),
		}
	}
}

impl From<(f32, f32)> for BoxLengths {
	fn from((horizontal, vertical): (f32, f32)) -> BoxLengths {
		BoxLengths { horizontal: horizontal.into(), vertical: vertical.into() }
	}
}

impl From<(AxisLengths, AxisLengths)> for BoxLengths {
	fn from((horizontal, vertical): (AxisLengths, AxisLengths)) -> BoxLengths {
		BoxLengths { horizontal, vertical }
	}
}