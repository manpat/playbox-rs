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
	Tetrahedron,
	Planes,
	Billboards,
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
	let mut backface_culling = false;

	let mut colors = [
		Color::white(),
		Color::grey(0.1),

		Color::rgb(1.0, 0.4, 0.7),
		Color::rgb(0.7, 0.4, 1.0),
		Color::rgb(0.4, 0.7, 1.0),
		Color::rgb(1.0, 0.7, 0.4),
		Color::rgb(0.4, 1.0, 0.7),
		Color::rgb(0.7, 1.0, 0.4),
	];

	let mut time = 0.0f32;

	engine.imgui.set_visible(true);
	engine.imgui.set_input_enabled(true);

	while !global_controller.should_quit() {
		time += 1.0/60.0;

		global_controller.update(&mut engine);

		let drag_camera = engine.input.frame_state().active(actions.drag_camera);
		engine.input.set_context_active(mouse_actions.context_id(), drag_camera);


		let ui = engine.imgui.frame();
		if let Some(_window) = imgui::Window::new("Mesh Builder").begin(ui) {
			if let Some(_) = ui.begin_combo("Mode", format!("{current_mode:?}")) {
				let modes = [
					Mode::Cubes,
					Mode::Tetrahedron,
					Mode::Planes,
					Mode::Billboards,
				];

				for mode in modes {
					if imgui::Selectable::new(format!("{mode:?}")).build(ui) {
						current_mode = mode;
					}
				}
			}

			ui.checkbox("Cull backfaces", &mut backface_culling);

			for (idx, color) in colors.iter_mut().enumerate() {
				let Color { r, g, b, .. } = color.to_srgb();
				let mut color_raw = [r, g, b];
				if imgui::ColorEdit::new(&format!("##{idx}"), &mut color_raw)
					.build(ui)
				{
					*color = Color::from(color_raw).to_linear();
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
			use gfx::traits::BuildableGeometry2D;
			use gfx::traits::BuildableGeometry3D;

			mesh_data_3d.clear();
			
			let mut mb = gfx::ColorMeshBuilder::new(&mut mesh_data_3d);
			mb.set_color(colors[0]);
			mb.on_plane_ref(gfx::OrthogonalOrientation::PositiveY)
				.build(Quad::unit().uniform_scale(4.0));

			mb.set_color(colors[2]);

			match current_mode {
				Mode::Cubes => {
					mb.build(Cuboid::unit().translate(Vec3::from_y(1.0)));

					mb.set_color(colors[3]);

					Cuboid::unit()
						.uniform_scale(0.5)
						.translate(Vec3::from_y(1.75))
						.build(&mut mb);

					mb.set_color(colors[4]);

					Cuboid::unit()
						.scale(Vec3::new(2.0, 0.2, 0.2))
						.translate(Vec3::new(0.0, 1.0, 0.6))
						.build(&mut mb);

					Cuboid::unit()
						.scale(Vec3::new(2.0, 0.2, 0.2))
						.translate(Vec3::new(0.0, 1.0, -0.6))
						.build(&mut mb);

					mb.set_color(colors[5]);
					Cuboid::unit()
						.uniform_scale(0.25)
						.rotate_x(TAU/8.0)
						.rotate_z(TAU/8.0)
						.translate(Vec3::from_y(2.0))
						.build(&mut mb);
				}

				Mode::Tetrahedron => {
					mb.build(Tetrahedron::unit().translate(Vec3::from_y(0.0)));

					mb.set_color(colors[3]);
					Tetrahedron::from_matrix(Mat3x4::rotate_y(PI*time))
						.uniform_scale(0.5)
						.rotate_x(TAU/2.0)
						.translate(Vec3::from_y(1.5))
						.build(&mut mb);

					mb.set_color(colors[4]);
					Tetrahedron::unit()
						.rotate_x(TAU/4.0)
						.scale(Vec3::new(0.2, 0.2, 1.0))
						.translate(Vec3::new(0.0, 1.0, 0.5))
						.build(&mut mb);

					Tetrahedron::unit()
						.rotate_x(-TAU/4.0)
						.scale(Vec3::new(0.2, 0.2, 1.0))
						.translate(Vec3::new(0.0, 1.0, -0.5))
						.build(&mut mb);
				}

				Mode::Planes => {
					use gfx::{BuilderSurface, OrthogonalOrientation};

					mb.on_plane_ref(OrthogonalOrientation::PositiveX)
						.build(Quad::unit().translate(Vec2::from_y(1.0)));

					mb.set_color(colors[3]);
					mb.on_plane_ref(OrthogonalOrientation::PositiveZ.to_surface_with_origin(Vec3::from_y(0.5)))
						.build(Quad::unit());
					mb.on_plane_ref(OrthogonalOrientation::NegativeZ.to_surface_with_origin(Vec3::new(0.0, 0.5, -0.2)))
						.build(Quad::unit().uniform_scale(0.5));

					mb.set_color(colors[4]);
					let mut pmb = mb.on_plane_ref(BuilderSurface::from_orthogonal(OrthogonalOrientation::PositiveY).with_origin(Vec3::from_y(0.5)));
					Quad::unit().uniform_scale(0.5).build(&mut pmb);

					pmb.set_color(colors[5]);
					Quad::unit().uniform_scale(0.25).translate(Vec2::new(0.5, 0.0)).build(&mut pmb);
					Quad::unit().uniform_scale(0.25).translate(Vec2::new(0.0, 0.5)).build(&mut pmb);

					mb.set_color(colors[6]);
					let mut pmb = mb.on_plane_ref(BuilderSurface::from(Quat::from_yaw(time/3.0)).with_origin(Vec3::from_y(2.0)));
					Polygon::unit(3).rotate(time).build(&mut pmb);

					pmb.set_color(colors[7]);
					Polygon::unit(5).rotate(time * 1.4).uniform_scale(1.1).translate(Vec2::from_x(1.0)).build(&mut pmb);

					pmb.set_color(colors[5]);
					Polygon::unit(7).rotate(time * 1.7).uniform_scale(1.2).translate(Vec2::from_x(-1.0)).build(&mut pmb);
				}

				Mode::Billboards => {
					use gfx::{BuilderSurface, OrthogonalOrientation};

					let camera_orientation = Quat::from_yaw(camera_yaw)
						* Quat::from_pitch(camera_pitch);

					let cam_right = camera_orientation.right();
					let cam_up = camera_orientation.up();
					let real_up = Vec3::from_y(1.0);

					let mut pmb = mb.on_plane_ref(camera_orientation);
					pmb.set_origin(Vec3::new(1.0, 0.0, 1.0));
					pmb.build(Quad::unit().translate(Vec2::from_y(0.5)));

					pmb.set_color(colors[3]);
					pmb.set_origin(Vec3::new(-1.0, 0.0, -1.0));
					pmb.build(Polygon::unit(13).uniform_scale(time.sin() * 0.3 + 0.5).translate(Vec2::from_y(0.5)));

					let mut pmb = mb.on_plane_ref(BuilderSurface::from_bases(cam_right, real_up));
					pmb.set_color(colors[4]);
					pmb.build(Quad::unit().scale(Vec2::new(1.0, 2.0)).translate(Vec2::from_y(1.0)));

					let mut pmb = mb.on_plane_ref(BuilderSurface::from_bases(cam_right, cam_up));
					pmb.set_color(colors[5]);
					pmb.set_origin(Vec3::new(1.0, 1.5, -1.0));
					pmb.build(Polygon::unit(5).rotate(-time*3.0));

					pmb.set_color(colors[6]);
					pmb.set_origin(Vec3::new(-1.0, 1.5, 1.0));
					pmb.build(Polygon::unit(3).rotate(time*1.3));
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
		gfx.set_clear_color(colors[1]);
		gfx.clear(gfx::ClearMode::ALL);
		gfx.set_backface_culling(backface_culling);

		gfx.bind_uniform_buffer(0, std_uniform_buffer);
		gfx.bind_shader(color_shader);

		mesh_3d.draw(&mut gfx, gfx::DrawMode::Triangles);

		engine = next_frame(engine).await;
	}

	Ok(())
}
