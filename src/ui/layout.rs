use crate::prelude::*;
use super::*;

pub type LayoutKey = usize;

struct WidgetLayout {
	layout_type: LayoutType,
	constraints: WidgetConstraints,

	size: Vec2,
	position: Vec2,
}

#[derive(Default)]
pub struct LayoutTree {
	widgets: Vec<WidgetLayout>,
	children: Vec<SmallVec<[LayoutKey; 4]>>,
}



pub fn layout_widget_tree(widget_tree: &mut WidgetTree) {
	let mut layout_tree = LayoutTree {
		widgets: Vec::with_capacity(widget_tree.widgets.len()),
		children: vec![const{ SmallVec::new_const() }; widget_tree.widgets.len()],
	};

	// Fill layout tree initial state - in reverse submission order
	for &widget_id in widget_tree.submission_order.iter().rev() {
		let layout_key = layout_tree.widgets.len();

		{
			let widget = widget_tree.widgets.get_mut(&widget_id).unwrap();
			layout_tree.widgets.push(WidgetLayout {
				constraints: widget.constraints.clone(),
				layout_type: widget.layout_type,

				size: Vec2::zero(),
				position: Vec2::zero(),
			});

			widget.layout_key = Some(layout_key);
		}

		let Some(children) = widget_tree.children.get(&widget_id)
			else { continue };

		if children.is_empty() {
			continue;
		}

		let layout_children = &mut layout_tree.children[layout_key];
		for child_id in children {
			let child_layout_key = widget_tree.widgets[child_id].layout_key.unwrap();
			layout_children.push(child_layout_key);
		}
	}

	// Measure from bottom up.
	for layout_key in 0..layout_tree.widgets.len() {
		let layout_children = &layout_tree.children[layout_key];
		if layout_children.is_empty() {
			continue;
		}

		let (leaf_nodes, post) = layout_tree.widgets.split_at_mut(layout_key);
		let widget = &mut post[0];

		match widget.layout_type {
			LayoutType::Stack => {
				let horizontal_measurement = measure_axis_overlapping(leaf_nodes, layout_children, Axis::Horizontal);
				let vertical_measurement = measure_axis_overlapping(leaf_nodes, layout_children, Axis::Vertical);

				adjust_constraints(widget.constraints.axis_mut(Axis::Horizontal), &horizontal_measurement);
				adjust_constraints(widget.constraints.axis_mut(Axis::Vertical), &vertical_measurement);
			}

			LayoutType::LeftToRight => unimplemented!(),
			LayoutType::RightToLeft => unimplemented!(),
			LayoutType::TopToBottom => unimplemented!(),
			LayoutType::BottomToTop => unimplemented!(),
		}
	}

	// Root should already be at the appropriate size.
	{
		let mut root = layout_tree.widgets.last_mut().unwrap();
		root.size = root.constraints.preferred_size();
		root.position = Vec2::zero();
	}

	// Size and position from top up.
	for layout_key in (0..layout_tree.widgets.len()).rev() {
		let layout_children = &layout_tree.children[layout_key];
		if layout_children.is_empty() {
			continue;
		}

		let (leaf_nodes, post) = layout_tree.widgets.split_at_mut(layout_key);
		let container = &post[0];

		match container.layout_type {
			LayoutType::Stack => {
				layout_axis_overlapping(leaf_nodes, layout_children, Axis::Horizontal, container);
				layout_axis_overlapping(leaf_nodes, layout_children, Axis::Vertical, container);
			}

			LayoutType::LeftToRight => unimplemented!(),
			LayoutType::RightToLeft => unimplemented!(),
			LayoutType::TopToBottom => unimplemented!(),
			LayoutType::BottomToTop => unimplemented!(),
		}
	}

	// Write back into widget tree
	for widget in widget_tree.widgets.values_mut() {
		let layout_key = widget.layout_key.unwrap();
		let layout = &layout_tree.widgets[layout_key];

		widget.rect = Some(Aabb2::from_min_size(layout.position, layout.size));
	}
}

struct AxisMeasurement {
	min_length: f32,
	preferred_length: f32,
}

fn measure_axis_overlapping(widgets: &[WidgetLayout], children: &[LayoutKey], axis: Axis) -> AxisMeasurement {
	let mut min_length = 0.0f32;
	let mut preferred_length = 0.0f32;

	for &key in children {
		let constraints = widgets[key].constraints.axis(axis);

		let margin_total = constraints.margin_start + constraints.margin_end;

		min_length = min_length.max(constraints.min + margin_total);
		preferred_length = preferred_length.max(constraints.preferred + margin_total);
	}

	AxisMeasurement {
		min_length,
		preferred_length,
	}
}

fn adjust_constraints(constraints: &mut WidgetAxisConstraints, measurement: &AxisMeasurement) {
	let initial_min = constraints.min;
	let initial_preferred = constraints.preferred;
	let padding_total = constraints.padding_start + constraints.padding_end;

	constraints.min = initial_min.max(measurement.min_length + padding_total).min(constraints.max);

	constraints.preferred = initial_preferred.max(measurement.preferred_length + padding_total)
		.clamp(constraints.min, constraints.max);
}

fn layout_axis_overlapping(widgets: &mut [WidgetLayout], children: &[LayoutKey], axis: Axis, container: &WidgetLayout) {
	let container_constraints = container.constraints.axis(axis);
	let container_size = length(&container.size, axis);
	let container_position = length(&container.position, axis);
	let container_padding = container_constraints.padding_start + container_constraints.padding_end;

	let available_content_size = (container_size - container_padding).max(0.0);
	let available_content_start = container_position + container_constraints.padding_start;

	for &key in children {
		let layout = &mut widgets[key];

		let constraints = layout.constraints.axis(axis);
		let margin_total = constraints.margin_start + constraints.margin_end;

		let min_length = constraints.min;
		let max_length = constraints.max;

		let layout_position = length_mut(&mut layout.position, axis);
		let layout_size = length_mut(&mut layout.size, axis);

		*layout_size = (available_content_size - margin_total).clamp(min_length, max_length);

		// TODO(pat.m): alignment

		*layout_position = available_content_start + (available_content_size - *layout_size) / 2.0;
	}
}


fn length_mut(v: &mut Vec2, axis: Axis) -> &mut f32 {
	match axis {
		Axis::Horizontal => &mut v.x,
		Axis::Vertical => &mut v.y,
	}
}

fn length(v: &Vec2, axis: Axis) -> f32 {
	match axis {
		Axis::Horizontal => v.x,
		Axis::Vertical => v.y,
	}
}