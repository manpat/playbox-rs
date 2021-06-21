use toybox::prelude::*;
use std::error::Error;

mod views;
mod model;
mod controller;

fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let mut engine = toybox::Engine::new("playbox")?;

	engine.gl_ctx.add_shader_import("global", include_str!("shaders/global.common.glsl"));

	let uniform_buffer = engine.gl_ctx.new_buffer();
	engine.gl_ctx.bind_uniform_buffer(0, uniform_buffer);


	use toybox::input;

	let mut global_input_ctx = engine.input.new_context("Global");
	let quit_action = global_input_ctx.new_trigger("Quit", input::raw::Scancode::Escape);
	let toggle_wireframe_action = global_input_ctx.new_trigger("Toggle Wireframe", input::raw::Scancode::Z);
	let play_sound_action = global_input_ctx.new_trigger("Play Sound", input::raw::Scancode::Num1);
	let play_stereo_sound_action = global_input_ctx.new_trigger("Play Stereo Sound", input::raw::Scancode::Num2);
	let play_static_stream_sound_action = global_input_ctx.new_trigger("Play Static Streamed Sound", input::raw::Scancode::Num3);
	let play_file_stream_sound_action = global_input_ctx.new_trigger("Play File Streamed Sound", input::raw::Scancode::Num4);
	let global_input_ctx = global_input_ctx.build();
	engine.input.enter_context(global_input_ctx);


	let pluck_sound_id = {
		let framerate = 44100;
		let freq = 440.0;

		let attack_t = framerate as f32 * 0.01;
		let release_t = framerate as f32 * 0.2;

		let sound_t = attack_t + release_t;
		let buffer_size = sound_t as usize;

		let samples = (0..buffer_size)
			.map(move |x| {
				let x = x as f32;
				let attack = (x / attack_t).min(1.0);
				let release = (1.0 - (x - attack_t) / (sound_t - attack_t)).powf(10.0);

				let envelope = attack*release;

				(x * freq / framerate as f32 * PI).sin() * envelope
			});

		let buffer = toybox::audio::Buffer::from_mono_samples(samples);
		engine.audio.register_buffer(buffer)
	};

	let stereo_sound_id = {
		let framerate = 44100;
		let freq = 660.0;

		let attack_t = framerate as f32 * 0.01;
		let release_t = framerate as f32 * 4.0;

		let sound_t = attack_t + release_t;
		let buffer_size = sound_t as usize;

		let samples = (0..buffer_size)
			.map(move |x| {
				let x = x as f32;
				let attack = (x / attack_t).min(1.0);
				let release = (1.0 - (x - attack_t) / (sound_t - attack_t)).powf(10.0);

				let envelope = attack*release;

				(x * freq / framerate as f32 * PI).sin() * envelope
			})
			.flat_map(|sample| [sample, -sample]);

		let buffer = toybox::audio::Buffer::from_stereo_samples(samples);
		engine.audio.register_buffer(buffer)
	};

	let static_ogg_sound_id = {
		let raw_data = include_bytes!("../assets/forest.ogg");
		let stream = toybox::audio::Stream::from_vorbis_static(raw_data)?;
		engine.audio.register_stream(stream)
	};

	let file_ogg_sound_id = {
		let stream = toybox::audio::Stream::from_vorbis_file("assets/forest.ogg")?;
		engine.audio.register_stream(stream)
	};


	let cube_view = views::CubeView::new(&engine.gl_ctx)?;
	let mut perf_view = views::PerfView::new(&engine.gl_ctx)?;


	let mut player = model::Player::new();
	let mut camera = model::Camera::new();

	let player_controller = controller::PlayerController::new(&mut engine.input);

	let mut wireframe_enabled = false;

	'main: loop {
		engine.process_events();

		if engine.should_quit() {
			break 'main
		}

		if engine.input.frame_state().active(quit_action) {
			break 'main
		}

		if engine.input.frame_state().active(toggle_wireframe_action) {
			wireframe_enabled = !wireframe_enabled;
			engine.gl_ctx.set_wireframe(wireframe_enabled);
		}

		if engine.input.frame_state().active(play_sound_action) {
			engine.audio.play_one_shot(pluck_sound_id);
		}

		if engine.input.frame_state().active(play_stereo_sound_action) {
			engine.audio.play_one_shot(stereo_sound_id);
		}

		if engine.input.frame_state().active(play_static_stream_sound_action) {
			engine.audio.play_one_shot(static_ogg_sound_id);
		}

		if engine.input.frame_state().active(play_file_stream_sound_action) {
			engine.audio.play_one_shot(file_ogg_sound_id);
		}

		{
			let Vec2{x, y} = engine.gl_ctx.canvas_size().to_vec2();
			camera.aspect = x / y;
		}

		player_controller.update(&mut engine.input, &mut player, &mut camera);

		let uniforms = Uniforms {
			projection_view: {
				let camera_orientation = Mat4::rotate_y(player.yaw) * Mat4::rotate_x(camera.pitch);

				Mat4::perspective(PI/3.0, camera.aspect, 0.1, 1000.0)
					* Mat4::translate(Vec3::from_z(-camera.zoom))
					* camera_orientation.inverse()
					* Mat4::translate(-player.position)
			},

			ui_projection_view: {
				Mat4::scale(Vec3::new(1.0 / camera.aspect, 1.0, 1.0))
			}
		};

		uniform_buffer.upload(&[uniforms], gl::BufferUsage::Stream);


		perf_view.update(&engine.instrumenter, camera.aspect);


		unsafe {
			gl::raw::ClearColor(0.1, 0.1, 0.1, 1.0);
			gl::raw::Clear(gl::raw::COLOR_BUFFER_BIT | gl::raw::DEPTH_BUFFER_BIT);
		}

		let mut view_ctx = views::ViewContext::new(&engine.gl_ctx, &mut engine.instrumenter);

		cube_view.draw(&mut view_ctx);
		perf_view.draw(&mut view_ctx);

		engine.end_frame();
	}

	Ok(())
}




#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
	projection_view: Mat4,
	ui_projection_view: Mat4,
	// NOTE: align to Vec4s
}

