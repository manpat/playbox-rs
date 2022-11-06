use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};
use crate::shaders;

toybox::declare_input_context! {
	struct Actions "Dungeon" {
		state forward { "Forward" [input::Scancode::W] }
		state back { "Back" [input::Scancode::S] }
		state left { "Left" [input::Scancode::A] }
		state right { "Right" [input::Scancode::D] }
		pointer mouse { "Mouse" }

		trigger toggle_debug { "Debug" [input::Scancode::Grave] }
	}
}



pub async fn play() -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = crate::global_controller::GlobalController::new(&mut engine, resource_scope_token.id())?;

	let actions = Actions::new_active(&mut engine);


	let mut resource_ctx = engine.gfx.resource_context(&resource_scope_token);

	let color_shader = resource_ctx.new_simple_shader(
		crate::shaders::COLOR_3D_VERT,
		crate::shaders::FLAT_COLOR_FRAG,
	)?;

	let mut std_uniform_buffer = resource_ctx.new_buffer(gfx::BufferUsage::Stream);
	let scene_mesh = gfx::Mesh::from_mesh_data(&mut resource_ctx, &build_map());


	let mut player_position = Vec2::zero();
	let mut player_orientation = 0.0;


	while !global_controller.should_quit() {
		global_controller.update(&mut engine);


		let input_state = engine.input.frame_state();
		{
			let orientation = Mat2x3::rotate(player_orientation);
			let forward = -orientation.column_y();
			let move_speed = 6.0 / 60.0;
			let rot_speed = TAU / 2.0 / 60.0;

			if input_state.active(actions.forward) {
				player_position += move_speed * forward;
			}

			if input_state.active(actions.back) {
				player_position -= move_speed * forward;
			}

			if input_state.active(actions.left) {
				player_orientation -= rot_speed;
			}

			if input_state.active(actions.right) {
				player_orientation += rot_speed;
			}
		}


		let projection_view = {
			let camera_orientation = Mat4::rotate_y(player_orientation);
			let camera_position = player_position.to_x0y() + Vec3::from_y(1.0);

			Mat4::perspective(PI/3.0, engine.gfx.aspect(), 0.1, 1000.0)
				* camera_orientation
				* Mat4::translate(-camera_position)
		};

		std_uniform_buffer.upload_single(&shaders::StdUniforms {
			projection_view,
			projection_view_inverse: projection_view.inverse(),
			ui_projection_view: Mat4::identity(),
		});

		let mut gfx = engine.gfx.draw_context();
		gfx.set_clear_color(Color::grey(0.1));
		gfx.clear(gfx::ClearMode::ALL);

		gfx.bind_uniform_buffer(0, std_uniform_buffer);
		gfx.bind_shader(color_shader);

		scene_mesh.draw(&mut gfx, gfx::DrawMode::Triangles);

		engine = next_frame(engine).await;
	}

	Ok(())
}


fn build_map() -> gfx::MeshData<gfx::ColorVertex> {
	let mut mesh_data = gfx::MeshData::new();
	let mut mb = gfx::ColorMeshBuilder::new(&mut mesh_data);

	build_room(&mut mb, Vec2i::zero());
	build_room(&mut mb, Vec2i::new(0, -1));
	build_room(&mut mb, Vec2i::new(0, -2));
	build_room(&mut mb, Vec2i::new(1, -2));

	mesh_data
}


const ROOM_SIZE: f32 = 4.0;
const ROOM_HEIGHT: f32 = 2.0;

fn build_room(mb: &mut gfx::ColorMeshBuilder<&mut gfx::MeshData<gfx::ColorVertex>>, location: Vec2i) {
	use gfx::geom::*;
	use gfx::OrthogonalOrientation;

	let origin = location.to_vec2() * ROOM_SIZE;
	let origin = origin.to_x0y();

	mb.set_color(Color::white());
	mb.on_plane_ref(OrthogonalOrientation::PositiveY.to_surface_with_origin(origin + Vec3::from_y(0.0)))
		.build(Quad::unit().uniform_scale(ROOM_SIZE));

	mb.on_plane_ref(OrthogonalOrientation::NegativeY.to_surface_with_origin(origin + Vec3::from_y(ROOM_HEIGHT)))
		.build(Quad::unit().uniform_scale(ROOM_SIZE));
}