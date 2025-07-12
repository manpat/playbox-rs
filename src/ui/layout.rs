use crate::prelude::*;
use super::*;

pub type LayoutKey = usize;

struct ResolvedLayout {
	config: WidgetLayout,
	source_id: WidgetId,

	size: Vec2,
	position: Vec2,
}

#[derive(Default)]
struct LayoutTree {
	widgets: Vec<ResolvedLayout>,
	children: Vec<SmallVec<[LayoutKey; 4]>>,

	widget_to_layout: HashMap<WidgetId, LayoutKey>,
}



pub fn layout_widget_tree(widget_tree: &mut WidgetTree) {
	let mut layout_tree = LayoutTree {
		widgets: Vec::with_capacity(widget_tree.widget_count()),
		children: vec![const{ SmallVec::new_const() }; widget_tree.widget_count()],

		widget_to_layout: HashMap::default(),
	};

	// Fill layout tree initial state - in reverse submission order
	for &widget_id in widget_tree.submission_order.iter().rev() {
		let layout_key = layout_tree.widgets.len();

		layout_tree.widgets.push(ResolvedLayout {
			config: widget_tree.get_layout(widget_id),
			source_id: widget_id,

			size: Vec2::zero(),
			position: Vec2::zero(),
		});

		layout_tree.widget_to_layout.insert(widget_id, layout_key);

		let children = widget_tree.get_children(widget_id);
		if children.is_empty() {
			continue;
		}

		let layout_children = &mut layout_tree.children[layout_key];
		for child_widget_id in children {
			let child_layout_key = layout_tree.widget_to_layout[child_widget_id];
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

		// Constrain horizontally
		let horizontal_measurement = match container.config.layout_type {
			LayoutType::Stack | LayoutType::TopToBottom | LayoutType::BottomToTop => measure_axis_overlapping(leaf_nodes, layout_children, Axis::Horizontal),
			LayoutType::LeftToRight | LayoutType::RightToLeft => measure_axis_linear(leaf_nodes, layout_children, Axis::Horizontal, container.config.horizontal.spacing),
		};

		// Constrain vertically
		let vertical_measurement = match container.config.layout_type {
			LayoutType::Stack | LayoutType::LeftToRight | LayoutType::RightToLeft => measure_axis_overlapping(leaf_nodes, layout_children, Axis::Vertical),
			LayoutType::TopToBottom | LayoutType::BottomToTop => measure_axis_linear(leaf_nodes, layout_children, Axis::Vertical, container.config.vertical.spacing),
		};

		adjust_container_constraints(&mut container.config.horizontal, &horizontal_measurement);
		adjust_container_constraints(&mut container.config.vertical, &vertical_measurement);
	}

	// Root should already be at the appropriate size.
	{
		let root = layout_tree.widgets.last_mut().unwrap();
		root.size = root.config.each(|layout| layout.max).into();
		root.position = Vec2::zero();
	}

	// Size and position from top down.
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

			LayoutType::LeftToRight => { layout_axis_linear(leaf_nodes, layout_children, Axis::Horizontal, container, false) }
			LayoutType::RightToLeft => { layout_axis_linear(leaf_nodes, layout_children, Axis::Horizontal, container, true) }
		}

		// Layout vertically
		match container.config.layout_type {
			LayoutType::Stack | LayoutType::LeftToRight | LayoutType::RightToLeft => {
				layout_axis_overlapping(leaf_nodes, layout_children, Axis::Vertical, container);
			}

			LayoutType::TopToBottom => { layout_axis_linear(leaf_nodes, layout_children, Axis::Vertical, container, true) }
			LayoutType::BottomToTop => { layout_axis_linear(leaf_nodes, layout_children, Axis::Vertical, container, false) }
		}
	}

	// Write back into widget tree
	for resolved_layout in layout_tree.widgets.iter() {
		let widget = widget_tree.get_mut(resolved_layout.source_id);
		widget.rect = Aabb2::from_min_size(resolved_layout.position, resolved_layout.size);
	}
}

struct ContentsMeasurement {
	min_length: f32,
	preferred_length: f32,
}

fn measure_axis_overlapping(widgets: &[ResolvedLayout], children: &[LayoutKey], axis: Axis) -> ContentsMeasurement {
	let mut min_length = 0.0f32;
	let mut preferred_length = 0.0f32;

	for &key in children {
		let config = widgets[key].config.axis(axis);
		let margin_total = config.margin.total();

		min_length = min_length.max(config.min + margin_total);
		preferred_length = preferred_length.max(config.preferred + margin_total);
	}

	ContentsMeasurement {
		min_length,
		preferred_length,
	}
}

fn measure_axis_linear(widgets: &[ResolvedLayout], children: &[LayoutKey], axis: Axis, spacing: f32) -> ContentsMeasurement {
	let mut min_length = 0.0f32;
	let mut preferred_length = 0.0f32;

	for &key in children {
		let config = widgets[key].config.axis(axis);
		let margin_total = config.margin.total();

		// TODO(pat.m): collapse margins
		min_length += config.min + margin_total;
		preferred_length += config.preferred + margin_total;
	}

	if !children.is_empty() {
		let spacing_contribution = children.len().saturating_sub(1) as f32 * spacing;
		min_length += spacing_contribution;
		preferred_length += spacing_contribution;
	}

	ContentsMeasurement {
		min_length,
		preferred_length,
	}
}

fn adjust_container_constraints(container: &mut WidgetAxisLayout, contents: &ContentsMeasurement) {
	let padding_total = container.padding.total();

	if container.flags.contains(WidgetLayoutFlags::FIT_TO_CONTENTS) {
		container.max = (contents.preferred_length + padding_total).clamp(container.min, container.max);
		container.preferred = container.max;
	}

	container.min = (contents.min_length + padding_total).clamp(container.min, container.max);

	container.preferred = container.preferred.max(contents.preferred_length + padding_total)
		.clamp(container.min, container.max);
}

fn layout_axis_overlapping(widgets: &mut [ResolvedLayout], children: &[LayoutKey], axis: Axis, container: &ResolvedLayout) {
	let container_config = container.config.axis(axis);
	let container_size = length(&container.size, axis);
	let container_position = length(&container.position, axis);
	let container_padding = container_config.padding.total();

	let available_content_size = (container_size - container_padding).max(0.0);
	let available_content_start = container_position + container_config.padding.start;

	for &key in children {
		let child = &mut widgets[key];

		let child_config = child.config.axis(axis);
		let margin_total = child_config.margin.total();

		let min_length = child_config.min;
		let max_length = child_config.max;

		let alignment = child_config.alignment.unwrap_or(container_config.child_alignment);

		let child_position = length_mut(&mut child.position, axis);
		let child_size = length_mut(&mut child.size, axis);

		*child_size = (available_content_size - margin_total).clamp(min_length, max_length);

		match alignment {
			Alignment::Begin => {
				*child_position = available_content_start + child_config.margin.start;
			}
			Alignment::Center => {
				*child_position = available_content_start + (available_content_size - *child_size) / 2.0;
			}
			Alignment::End => {
				*child_position = available_content_start + available_content_size - *child_size - child_config.margin.end;
			}
		}
	}
}

fn layout_axis_linear(widgets: &mut [ResolvedLayout], children: &[LayoutKey], axis: Axis, container: &ResolvedLayout, reverse: bool) {
	let container_config = container.config.axis(axis);
	let container_size = length(&container.size, axis);
	let container_padding = container_config.padding.total();

	let available_space = (container_size - container_padding).max(0.0);
	let spacing_contribution = children.len().saturating_sub(1) as f32 * container_config.spacing;

	let mut total_min_size = spacing_contribution;
	let mut num_children_wanting_expand = 0;

	for &key in children {
		let config = &widgets[key].config.axis(axis);

		total_min_size += config.margin.total() + config.min;

		if config.max > config.min {
			num_children_wanting_expand += 1;
		}

		*length_mut(&mut widgets[key].size, axis) = config.min;
	}

	// TODO(pat.m): if layout has overflow, then remaining_space can be infinite
	let mut remaining_space = (available_space - total_min_size).max(0.0);
	while remaining_space > 0.0 && num_children_wanting_expand > 0 {
		let available_expansion_space = (remaining_space / num_children_wanting_expand as f32).max(0.0);

		for &key in children {
			let config = &widgets[key].config.axis(axis);
			let size = length_mut(&mut widgets[key].size, axis);

			let allowed_expansion = config.max - *size;
			if allowed_expansion <= 0.0 {
				// Already as big as we can get.
				continue;
			}

			let claimed_expansion = available_expansion_space.min(allowed_expansion);

			*size += claimed_expansion;
			remaining_space -= claimed_expansion;

			if available_expansion_space >= allowed_expansion {
				// We've reached our max, so we don't need to expand any more.
				num_children_wanting_expand -= 1;
			}

			// We have to check for denorms since we're subtracting increasingly smaller amounts from remaining_space.
			if remaining_space.is_subnormal() || remaining_space <= 0.0 {
				remaining_space = 0.0;
				break;
			}
		}
	}

	// Position
	let container_position = length(&container.position, axis);
	let mut content_position = container_position + container_config.padding.start;

	// TODO(pat.m): config.alignment

	if reverse {
		match container_config.child_alignment {
			Alignment::Begin => {
				content_position += remaining_space;
			}
			Alignment::Center => {
				content_position += remaining_space / 2.0;
			}
			Alignment::End => {}
		}

		for &key in children.iter().rev() {
			let config = &widgets[key].config.axis(axis);
			let size = length(&widgets[key].size, axis);

			content_position += config.margin.start;
			*length_mut(&mut widgets[key].position, axis) = content_position;

			content_position += size + config.margin.end + container_config.spacing;
		}
	} else {
		match container_config.child_alignment {
			Alignment::Begin => {}
			Alignment::Center => {
				content_position += remaining_space / 2.0;
			}
			Alignment::End => {
				content_position += remaining_space;
			}
		}

		for &key in children {
			let config = &widgets[key].config.axis(axis);
			let size = length(&widgets[key].size, axis);

			content_position += config.margin.start;
			*length_mut(&mut widgets[key].position, axis) = content_position;

			content_position += size + config.margin.end + container_config.spacing;
		}
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