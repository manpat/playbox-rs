use crate::prelude::*;
use super::*;

pub type LayoutKey = usize;

struct ResolvedLayout {
	config: WidgetLayout,

	size: Vec2,
	position: Vec2,
}

#[derive(Default)]
pub struct LayoutTree {
	widgets: Vec<ResolvedLayout>,
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
			layout_tree.widgets.push(ResolvedLayout {
				config: widget.layout.clone(),

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
		let container = &mut post[0];

		match container.config.layout_type {
			LayoutType::Stack => {
				let horizontal_measurement = measure_axis_overlapping(leaf_nodes, layout_children, Axis::Horizontal);
				let vertical_measurement = measure_axis_overlapping(leaf_nodes, layout_children, Axis::Vertical);

				adjust_container_constraints(container.config.axis_mut(Axis::Horizontal), &horizontal_measurement);
				adjust_container_constraints(container.config.axis_mut(Axis::Vertical), &vertical_measurement);
			}

			LayoutType::LeftToRight => unimplemented!(),
			LayoutType::RightToLeft => unimplemented!(),
			LayoutType::TopToBottom => unimplemented!(),
			LayoutType::BottomToTop => unimplemented!(),
		}
	}

	// Root should already be at the appropriate size.
	{
		let root = layout_tree.widgets.last_mut().unwrap();
		root.size = root.config.preferred_size();
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

		// Layout horizontally
		match container.config.layout_type {
			LayoutType::Stack | LayoutType::TopToBottom | LayoutType::BottomToTop => {
				layout_axis_overlapping(leaf_nodes, layout_children, Axis::Horizontal, container)
			}

			LayoutType::LeftToRight => { /*layout_axis_linear()*/ }
			LayoutType::RightToLeft => { /*layout_axis_linear()*/ }
		}

		// Layout vertically
		match container.config.layout_type {
			LayoutType::Stack | LayoutType::LeftToRight | LayoutType::RightToLeft => {
				layout_axis_overlapping(leaf_nodes, layout_children, Axis::Vertical, container);
			}

			LayoutType::TopToBottom => { /*layout_axis_linear()*/ }
			LayoutType::BottomToTop => { /*layout_axis_linear()*/ }
		}
	}

	// Write back into widget tree
	for widget in widget_tree.widgets.values_mut() {
		let layout_key = widget.layout_key.unwrap();
		let layout = &layout_tree.widgets[layout_key];

		widget.rect = Some(Aabb2::from_min_size(layout.position, layout.size));
	}
}

struct ContainerMeasurement {
	min_length: f32,
	preferred_length: f32,
}

fn measure_axis_overlapping(widgets: &[ResolvedLayout], children: &[LayoutKey], axis: Axis) -> ContainerMeasurement {
	let mut min_length = 0.0f32;
	let mut preferred_length = 0.0f32;

	for &key in children {
		let config = widgets[key].config.axis(axis);
		let margin_total = config.margin_start + config.margin_end;

		min_length = min_length.max(config.min + margin_total);
		preferred_length = preferred_length.max(config.preferred + margin_total);
	}

	ContainerMeasurement {
		min_length,
		preferred_length,
	}
}

fn adjust_container_constraints(constraints: &mut WidgetAxisLayout, measurement: &ContainerMeasurement) {
	let initial_min = constraints.min;
	let initial_preferred = constraints.preferred;
	let padding_total = constraints.padding_start + constraints.padding_end;

	constraints.min = initial_min.max(measurement.min_length + padding_total).min(constraints.max);

	constraints.preferred = initial_preferred.max(measurement.preferred_length + padding_total)
		.clamp(constraints.min, constraints.max);
}

fn layout_axis_overlapping(widgets: &mut [ResolvedLayout], children: &[LayoutKey], axis: Axis, container: &ResolvedLayout) {
	let container_config = container.config.axis(axis);
	let container_size = length(&container.size, axis);
	let container_position = length(&container.position, axis);
	let container_padding = container_config.padding_start + container_config.padding_end;

	let available_content_size = (container_size - container_padding).max(0.0);
	let available_content_start = container_position + container_config.padding_start;

	for &key in children {
		let child = &mut widgets[key];

		let child_config = child.config.axis(axis);
		let margin_total = child_config.margin_start + child_config.margin_end;

		let min_length = child_config.min;
		let max_length = child_config.max;

		let alignment = child_config.alignment.unwrap_or(container_config.child_alignment);

		let child_position = length_mut(&mut child.position, axis);
		let child_size = length_mut(&mut child.size, axis);

		*child_size = (available_content_size - margin_total).clamp(min_length, max_length);

		match alignment {
			Alignment::Start => {
				*child_position = available_content_start + child_config.margin_start;
			}
			Alignment::Center => {
				*child_position = available_content_start + (available_content_size - *child_size) / 2.0;
			}
			Alignment::End => {
				*child_position = available_content_start + available_content_size - *child_size - child_config.margin_end;
			}
		}
	}
}

// fn layout_axis_linear(widgets: &mut [ResolvedLayout], children: &[LayoutKey], axis: Axis, container: &ResolvedLayout) {

// }


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