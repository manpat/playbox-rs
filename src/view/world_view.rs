use crate::prelude::*;
use model::*;

mod room_renderer;
use room_renderer::*;

mod room_mesh_builder;
use room_mesh_builder::*;


// This fudge factor is to deal with the fact that we can look up and see behind us.
const BEHIND_VIEWER_BUFFER_DIST: f32 = 10.0;

// Max recursion depth when calculating room visibility.
const MAX_VISIBILITY_RECURSION_DEPTH: i32 = 50;


pub struct WorldView {
	room_renderer: RoomRenderer,

	// Every visible instance of each room
	visible_rooms: Vec<RoomInstance>,

	message_bus: MessageBus,
	change_subscription: Subscription<WorldChangedEvent>,
}

impl WorldView {
	pub fn new(gfx: &mut gfx::System, processed_world: &ProcessedWorld, message_bus: MessageBus) -> anyhow::Result<Self> {
		Ok(Self {
			room_renderer: RoomRenderer::new(gfx, processed_world)?,
			visible_rooms: Vec::new(),

			change_subscription: message_bus.subscribe(),
			message_bus,
		})
	}

	#[instrument(skip_all, name="world_view draw")]
	pub fn draw(&mut self, gfx: &mut gfx::System, processed_world: &ProcessedWorld, viewer_placement: Placement) {
		// Draw room you're in
		// then for each wall,
		// 	check if it has a neighbouring room, and if so
		// 	calculate transform between connected walls, and build that room,
		// 	using wall intersection to calculate a frustum to cull by

		if self.message_bus.any(&self.change_subscription) {
			self.room_renderer.rebuild(gfx, processed_world);
		}

		self.build_visibility_graph(processed_world, viewer_placement);

		// Draw
		// let mut group = gfx.frame_encoder.command_group(gfx::FrameStage::Main);

		for &RoomInstance{room_id, room_to_world, clip_by, height_offset} in self.visible_rooms.iter() {
			let [x,z,w] = room_to_world.columns();
			let transform = Mat3x4::from_columns([
				x.to_x0y(),
				Vec3::from_y(1.0),
				z.to_x0y(),
				w.to_x0y() + Vec3::from_y(height_offset)
			]);

			let planes = match clip_by {
				Some(ClipState{left_aperture, right_aperture, local_viewer_position, aperture_plane, ..}) => {
					let pos_to_left = left_aperture - local_viewer_position;
					let pos_to_right = right_aperture - local_viewer_position;

					let normal_a = pos_to_right.perp().normalize();
					let dist_a = local_viewer_position.dot(normal_a);

					let normal_b = -pos_to_left.perp().normalize();
					let dist_b = local_viewer_position.dot(normal_b);

					let plane_0 = normal_a.to_x0y().extend(dist_a);
					let plane_1 = normal_b.to_x0y().extend(dist_b);

					let [x, y, w] = aperture_plane.into();
					let plane_2 = Vec4::new(x, 0.0, y, w);

					[plane_0, plane_1, plane_2]
				}

				None => [Vec4::from_w(-1.0), Vec4::from_w(-1.0), Vec4::from_w(-1.0)],
			};

			self.room_renderer.draw(&mut gfx.frame_encoder, room_id, transform, &planes)
		}
	}

	#[instrument(skip_all, name="world_view build_visibility_graph")]
	fn build_visibility_graph(&mut self, processed_world: &ProcessedWorld, viewer_placement: Placement) {
		self.visible_rooms.clear();
		self.visible_rooms.push(RoomInstance {
			room_id: viewer_placement.room_id,
			room_to_world: Mat2x3::identity(),
			height_offset: 0.0,
			clip_by: None,
		});

		let viewer_forward = viewer_placement.forward();
		let geometry = processed_world.geometry();

		let mut instance_index = 0;

		// Build visibility graph
		while let Some(&RoomInstance{room_id, room_to_world, clip_by, height_offset}) = self.visible_rooms.get(instance_index) {
			instance_index += 1;

			let depth = clip_by.map_or(0, |c| c.depth);
			if depth >= MAX_VISIBILITY_RECURSION_DEPTH {
				continue
			}

			let local_viewer_position = clip_by.map_or(viewer_placement.position, |c| c.local_viewer_position);

			for wall_id in geometry.room_walls(room_id) {
				let Some(wall_info) = processed_world.wall_info(wall_id) else {
					continue
				};
				let Some(connection_info) = &wall_info.connection_info else {
					continue
				};

				let start_vertex = connection_info.aperture_start;
				let end_vertex = connection_info.aperture_end;

				// If the aperture we're considering isn't CCW from our position then cull it and the room it connects to.
				if (end_vertex - local_viewer_position).wedge(start_vertex - local_viewer_position) < 0.0 {
					continue;
				}

				// If the aperture is completely behind us then cull it and the room it connects to.
				let start_vertex_invisible = viewer_forward.dot(room_to_world * start_vertex) < -BEHIND_VIEWER_BUFFER_DIST;
				let end_vertex_invisible = viewer_forward.dot(room_to_world * end_vertex) < -BEHIND_VIEWER_BUFFER_DIST;

				if start_vertex_invisible && end_vertex_invisible {
					continue;
				}


				let (left_aperture, right_aperture, unclipped_left_aperture) = match &clip_by {
					Some(clip_state) => {
						match clip_wall_segment((start_vertex, end_vertex), clip_state) {
							Some((left, right)) => (left, right, start_vertex),
							None => continue,
						}
					}

					None => (start_vertex, end_vertex, start_vertex),
				};


				let total_transform = room_to_world * connection_info.target_to_source;

				// TODO(pat.m): this is kind of a mess, and wouldn't really be necessary if clip_wall_segment actually clipped things.
				// but it works
				let aperture_normal = connection_info.source_to_target * wall_info.normal.extend(0.0);
				let aperture_plane = aperture_normal.extend(aperture_normal.dot(connection_info.source_to_target * unclipped_left_aperture));

				self.visible_rooms.push(RoomInstance {
					room_id: connection_info.target_room,
					room_to_world: total_transform,
					height_offset: height_offset + connection_info.height_difference,

					clip_by: Some(ClipState {
						depth: depth+1,

						// All of these should be in the space of the target room
						local_viewer_position: connection_info.source_to_target * local_viewer_position,
						left_aperture: connection_info.source_to_target * left_aperture,
						right_aperture: connection_info.source_to_target * right_aperture,

						aperture_plane,
					})
				});
			}
		}
	}
}






#[derive(Debug, Copy, Clone, Default)]
struct ClipState {
	depth: i32,

	local_viewer_position: Vec2,
	left_aperture: Vec2,
	right_aperture: Vec2,
	aperture_plane: Vec3,
}

fn clip_wall_segment((mut left_vertex, mut right_vertex): (Vec2, Vec2), clip_by: &ClipState) -> Option<(Vec2, Vec2)> {
	let &ClipState{left_aperture, right_aperture, local_viewer_position, ..} = clip_by;

	let pos_to_left_clip = left_aperture - local_viewer_position;
	let pos_to_right_clip = right_aperture - local_viewer_position;

	let pos_to_left_vert = left_vertex - local_viewer_position;
	let pos_to_right_vert = right_vertex - local_viewer_position;

	// Full cull
	if pos_to_right_vert.wedge(pos_to_left_clip) < 0.0 {
		return None
	}

	if pos_to_left_vert.wedge(pos_to_right_clip) > 0.0 {
		return None
	}

	// Clip
	let wedge_product = pos_to_left_vert.wedge(pos_to_left_clip);
	if wedge_product < 0.0 {
		// TODO(pat.m): actually clip here - will help later
		left_vertex = left_aperture;
	}

	let wedge_product = pos_to_right_vert.wedge(pos_to_right_clip);
	if wedge_product > 0.0 {
		// TODO(pat.m): actually clip here - will help later
		right_vertex = right_aperture;
	}

	Some((left_vertex, right_vertex))
}


struct RoomInstance {
	room_id: RoomId,
	room_to_world: Mat2x3,
	height_offset: f32,
	clip_by: Option<ClipState>,
}

