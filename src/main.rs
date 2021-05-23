use toybox::prelude::*;
use std::error::Error;



fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;
	let mut instrumenter = toybox::perf::Instrumenter::new(&engine.gl_ctx);

	engine.gl_ctx.add_shader_import("global", include_str!("shaders/global.common.glsl"));


	let uniform_buffer = engine.gl_ctx.new_buffer();
	engine.gl_ctx.bind_uniform_buffer(0, uniform_buffer);



	let mut uniforms = Uniforms {
		projection_view: Mat4::ident(),
	};


	let mut aspect = 1.0f32;
	let mut zoom = 12.0f32;

	let mut yaw = 0.0f32;
	let mut pitch = -PI / 5.0;

	let mut camera_pos = Vec3::new(0.0, 2.0, 0.0);
	let mut forward_pressed = false;
	let mut back_pressed = false;
	let mut left_pressed = false;
	let mut right_pressed = false;
	let mut shift_pressed = false;

	let mut left_down = false;
	let mut right_down = false;

	let mut wireframe_enabled = false;

	let mut mouse_world_pos = Vec2::zero();

	'main: loop {
		for event in engine.event_pump.poll_iter() {
			use sdl2::event::{Event, WindowEvent};
			use sdl2::keyboard::Keycode;
			use sdl2::mouse::MouseButton;

			match event {
				Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'main,
				Event::Window{ win_event: WindowEvent::Resized(w, h), .. } => unsafe {
					gl::raw::Viewport(0, 0, w as _, h as _);
					aspect = w as f32 / h as f32;
				}

				Event::MouseWheel { y, .. } => {
					zoom = (zoom.log2() - y as f32 / 5.0).exp2();
				}

				Event::MouseMotion { xrel, yrel, x, y, .. } => {
					if left_down {
						yaw += xrel as f32 * 0.005;
						pitch = (pitch - yrel as f32 * 0.005).clamp(-PI, PI);
					}

					let (w, h) = engine.window.drawable_size();
					let mouse_x =  x as f32 / w as f32 * 2.0 - 1.0;
					let mouse_y = -(y as f32 / h as f32 * 2.0 - 1.0);

					let proj_view_inv = uniforms.projection_view.inverse();

					let near_point = proj_view_inv * Vec4::new(mouse_x, mouse_y, -1.0, 1.0);
					let near_point = near_point.to_vec3() / near_point.w;

					let far_point = proj_view_inv * Vec4::new(mouse_x, mouse_y, 1.0, 1.0);
					let far_point = far_point.to_vec3() / far_point.w;

					let ray_dir = (far_point - near_point).normalize();

					let plane = Plane::new(Vec3::from_y(1.0), 0.0);

					if plane.normal.dot(ray_dir).abs() > 0.01 {
						let t = (plane.length - plane.normal.dot(near_point)) / plane.normal.dot(ray_dir);
						let world_pos = near_point + ray_dir * t;

						mouse_world_pos = world_pos.to_xz();
					}
				}

				Event::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
					MouseButton::Left => { left_down = true }
					MouseButton::Right => { right_down = true }
					_ => {}
				}

				Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
					MouseButton::Left => { left_down = false }
					MouseButton::Right => { right_down = false }
					_ => {}
				}

				Event::KeyDown { keycode: Some(keycode), .. } => match keycode {
					Keycode::Z => {
						wireframe_enabled = !wireframe_enabled;
						engine.gl_ctx.set_wireframe(wireframe_enabled);
					}

					Keycode::W => { forward_pressed = true }
					Keycode::S => { back_pressed = true }
					Keycode::A => { left_pressed = true }
					Keycode::D => { right_pressed = true }
					Keycode::LShift => { shift_pressed = true }
					_ => {}
				}
				Event::KeyUp { keycode: Some(keycode), .. } => match keycode {
					Keycode::W => { forward_pressed = false }
					Keycode::S => { back_pressed = false }
					Keycode::A => { left_pressed = false }
					Keycode::D => { right_pressed = false }
					Keycode::LShift => { shift_pressed = false }
					_ => {}
				}
				_ => {}
			}
		}


		let camera_yaw_mat = Mat4::yrot(yaw);
		let camera_orientation = camera_yaw_mat * Mat4::xrot(pitch);

		let move_speed = match shift_pressed {
			true => 15.0,
			false => 5.0,
		};

		let cam_move_fwd = camera_yaw_mat * Vec3::from_z(-move_speed / 60.0);
		let cam_move_right = camera_yaw_mat * Vec3::from_x(move_speed / 60.0);

		if forward_pressed { camera_pos += cam_move_fwd }
		if back_pressed { camera_pos -= cam_move_fwd }
		if left_pressed { camera_pos -= cam_move_right }
		if right_pressed { camera_pos += cam_move_right }



		uniforms = Uniforms {
			projection_view: {
				Mat4::perspective(PI/3.0, aspect, 0.1, 1000.0)
					* Mat4::translate(Vec3::from_z(-zoom))
					* camera_orientation.inverse()
					* Mat4::translate(-camera_pos)
			}
		};

		uniform_buffer.upload(&[uniforms], gl::BufferUsage::Stream);



		unsafe {
			gl::raw::ClearColor(0.1, 0.1, 0.1, 1.0);
			gl::raw::Clear(gl::raw::COLOR_BUFFER_BIT | gl::raw::DEPTH_BUFFER_BIT);
		}


		instrumenter.end_frame();
		engine.window.gl_swap_window();
	}

	Ok(())
}




#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
	projection_view: Mat4,
	// NOTE: align to Vec4s
}

