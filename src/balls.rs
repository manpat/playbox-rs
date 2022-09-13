use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};
use crate::shaders;


pub async fn play() -> Result<(), Box<dyn Error>> {
	let mut engine = start_loop().await;
	let resource_scope_token = engine.new_resource_scope();

	let mut global_controller = crate::platformer3d::controller::GlobalController::new(&mut engine, resource_scope_token.id())?;


	let mut resource_ctx = engine.gfx.resource_context(&resource_scope_token);
	let mut mesh = gfx::Mesh::new(&mut resource_ctx);
	let mut mesh_data: gfx::MeshData<gfx::ColorVertex> = gfx::MeshData::new();

	let shader = resource_ctx.new_simple_shader(shaders::COLOR_3D_VERT, shaders::FLAT_COLOR_FRAG)?;
	let mut uniform_buffer = resource_ctx.new_buffer(gfx::BufferUsage::Stream);

	let mut camera = Camera {
		pos: Vec2::new(0.0, 3.0),
		elevation: 1.0,
		yaw: 0.0,
	};

	loop {
		global_controller.update(&mut engine);
		if global_controller.should_quit() {
			break
		}

		camera.yaw += 1.0/60.0;
		camera.pos = Vec2::from_angle(camera.yaw + PI/2.0) * 3.0;

		use gfx::geom::*;

		let mut mb = gfx::ColorMeshBuilder::new(&mut mesh_data)
			.on_plane(Mat3::identity());

		mb.set_color(Color::hsv(80.0, 0.6, 0.6));
		mb.build(Polygon::unit(18));

		mb.set_color(Color::hsv(130.0, 0.6, 0.6));
		mb.build(Polygon::from_pos_scale(3, Vec2::new(-1.5, 1.0), Vec2::splat(1.0)));

		mesh.upload(&mesh_data);


		let uniforms = build_uniforms(&camera, engine.gfx.aspect());
		uniform_buffer.upload_single(&uniforms);

		let mut gfx = engine.gfx.draw_context();
		gfx.set_clear_color(Color::hsv(190.0, 0.3, 0.8));
		gfx.clear(gfx::ClearMode::ALL);

		gfx.bind_uniform_buffer(0, uniform_buffer);
		gfx.bind_shader(shader);

		mesh.draw(&mut gfx, gfx::DrawMode::Triangles);

		engine = next_frame(engine).await;
	}

	Ok(())
}



struct Camera {
	pos: Vec2,
	elevation: f32,

	yaw: f32,
}




#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
	projection_view: Mat4,
	projection_view_inverse: Mat4,
	ui_projection_view: Mat4,
	// NOTE: align to Vec4s
}


fn build_uniforms(camera: &Camera, aspect: f32) -> Uniforms {
	let eye_pos = camera.pos.to_x0z() + Vec3::from_y(camera.elevation);

	let projection_view = {
		Mat4::perspective(PI/3.0, aspect, 0.1, 1000.0)
			* Mat4::rotate_y_translate(-camera.yaw, -eye_pos)
	};

	Uniforms {
		projection_view,
		projection_view_inverse: projection_view.inverse(),

		ui_projection_view: Mat4::identity()
	}
}