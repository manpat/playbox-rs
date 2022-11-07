use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};
use crate::shaders;

toybox::declare_input_context! {
	struct Actions "Mesh Builder Test" {
		state drag_camera { "Drag Camera" [input::MouseButton::Left] }

		trigger toggle_debug { "Debug" [input::Scancode::Grave] }
	}
}


toybox::declare_input_context! {
	struct MouseActions "Mesh Builder Test Mouse" {
		mouse mouse { "Mouse" [1.0] }
	}
}



#[derive(Copy, Clone, Debug)]
enum Mode {
	Cubes,
}



pub async fn play() -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = crate::global_controller::GlobalController::new(&mut engine, resource_scope_token.id())?;

	let actions = Actions::new_active(&mut engine);
	let mouse_actions = MouseActions::new(&mut engine);


	let mut resource_ctx = engine.gfx.resource_context(&resource_scope_token);

	let color_shader = resource_ctx.new_simple_shader(
		crate::shaders::COLOR_3D_VERT,
		crate::shaders::FLAT_COLOR_FRAG,
	)?;

	let mut mesh_3d = gfx::Mesh::new(&mut resource_ctx);
	// let mesh_2d = Mesh::new(&mut resource_ctx);

	let mut mesh_data_3d = gfx::MeshData::new();
	// let mesh_data_2d = MeshData::new();

	let mut std_uniform_buffer = resource_ctx.new_buffer(gfx::BufferUsage::Stream);


	let mut camera_yaw = 0.0f32;
	let mut camera_pitch = 0.0f32;

	let mut camera_yaw_vel = 0.0f32;
	let mut camera_pitch_vel = 0.0f32;

	let mut current_mode = Mode::Cubes;

	engine.imgui.set_visible(true);
	engine.imgui.set_input_enabled(true);

	while !global_controller.should_quit() {
		global_controller.update(&mut engine);

		let drag_camera = engine.input.frame_state().active(actions.drag_camera);
		engine.input.set_context_active(mouse_actions.context_id(), drag_camera);


		let ui = engine.imgui.frame();
		if let Some(_window) = imgui::Window::new("Mesh Builder").begin(ui) {
			if let Some(_) = ui.begin_combo("Mode", format!("{current_mode:?}")) {
				let modes = [
					Mode::Cubes,
				];

				for mode in modes {
					if imgui::Selectable::new(format!("{mode:?}")).build(ui) {
						current_mode = mode;
					}
				}
			}
		}


		let dt = 10.0 / 60.0;

		let drag_delta = engine.input.frame_state().mouse(mouse_actions.mouse).unwrap_or(Vec2::zero()) * Vec2::new(-1.0, 1.0);
		if drag_delta.x.abs() > camera_yaw_vel.abs() {
			camera_yaw_vel = drag_delta.x;
		} else {
			camera_yaw_vel = dt.lerp(camera_yaw_vel, drag_delta.x);
		}

		if drag_delta.y.abs() > camera_pitch_vel.abs() {
			camera_pitch_vel = drag_delta.y;
		} else {
			camera_pitch_vel = dt.lerp(camera_pitch_vel, drag_delta.y);
		}

		camera_yaw = (0.5f32).lerp(camera_yaw, camera_yaw + camera_yaw_vel);
		camera_pitch = (0.5f32).lerp(camera_pitch, (camera_pitch + camera_pitch_vel).clamp(-TAU/4.0, 0.2));


		{
			use gfx::geom::*;
			use gfx::traits::BuildableGeometry3D;

			mesh_data_3d.clear();
			
			let mut mb = gfx::ColorMeshBuilder::new(&mut mesh_data_3d);

			mb.on_plane_ref(gfx::OrthogonalOrientation::PositiveY)
				.build(Quad::unit().uniform_scale(4.0));

			mb.set_color(Color::rgb(1.0, 0.4, 0.7));

			match current_mode {
				Mode::Cubes => {
					mb.build(Cuboid::unit().translate(Vec3::from_y(1.0)));

					mb.set_color(Color::rgb(0.7, 0.4, 1.0));

					Cuboid::unit()
						.uniform_scale(0.5)
						.translate(Vec3::from_y(1.75))
						.build(&mut mb);

					mb.set_color(Color::rgb(0.4, 0.7, 1.0));

					Cuboid::unit()
						.scale(Vec3::new(2.0, 0.2, 0.2))
						.translate(Vec3::new(0.0, 1.0, 0.6))
						.build(&mut mb);

					Cuboid::unit()
						.scale(Vec3::new(2.0, 0.2, 0.2))
						.translate(Vec3::new(0.0, 1.0, -0.6))
						.build(&mut mb);

					Cuboid::unit()
						.uniform_scale(0.25)
						.rotate_x(TAU/8.0)
						.rotate_z(TAU/8.0)
						.translate(Vec3::from_y(2.0))
						.build(&mut mb);
				}
			}

			mesh_3d.upload(&mesh_data_3d);
		}



		let projection_view = {
			let camera_orientation = Quat::from_yaw(camera_yaw)
				* Quat::from_pitch(camera_pitch);

			let camera_position = camera_orientation.forward() * -3.0 + Vec3::from_y(1.0);

			Mat4::perspective(PI/3.0, engine.gfx.aspect(), 0.1, 1000.0)
				* camera_orientation.conjugate().to_mat4()
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
		gfx.set_backface_culling(true);

		gfx.bind_uniform_buffer(0, std_uniform_buffer);
		gfx.bind_shader(color_shader);

		mesh_3d.draw(&mut gfx, gfx::DrawMode::Triangles);

		engine = next_frame(engine).await;
	}

	Ok(())
}
