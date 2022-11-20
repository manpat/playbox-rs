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
	let mut debug_active = false;


	let mut resource_ctx = engine.gfx.resource_context(&resource_scope_token);

	let color_shader = resource_ctx.new_simple_shader(
		crate::shaders::TEX_3D_VERT,
		crate::shaders::TEXTURED_FRAG,
	)?;

	let mut std_uniform_buffer = resource_ctx.new_buffer(gfx::BufferUsage::Stream);
	let mut std_uniform_buffer_ui = resource_ctx.new_buffer(gfx::BufferUsage::Stream);
	let scene_mesh = gfx::Mesh::from_mesh_data(&mut resource_ctx, &build_map());

	let mut dynamic_mesh = gfx::Mesh::new(&mut resource_ctx);
	let mut ui_mesh = gfx::Mesh::new(&mut resource_ctx);
	let mut mesh_data = gfx::MeshData::new();


	let (texture, _image_size) = load_texture(&mut resource_ctx, "assets/dungeon.png")?;



	let mut player_position = Vec2::zero();
	let mut player_orientation = 0.0;

	let mut player_has_sword = false;
	let mut player_has_potion = false;

	let sword_pos = Vec2::new(0.0, -5.0);
	let potion_pos = Vec2::new(2.0, -8.0);

	let mut time = 0.0;


	while !global_controller.should_quit() {
		time += 1.0 / 60.0;

		global_controller.update(&mut engine);

		{
			engine.imgui.set_input_enabled(debug_active);
			engine.imgui.set_visible(debug_active);

			let ui = engine.imgui.frame();

			if let Some(_window) = imgui::Window::new("Dungeon").begin(&ui) {
				ui.checkbox("Sword", &mut player_has_sword);
				ui.checkbox("Potion", &mut player_has_potion);


				let id = toybox::imgui_backend::texture_key_to_imgui_id(texture);

				let window_width = ui.window_size()[0];
				let image_size = Vec2::splat(window_width - 50.0);

				imgui::Image::new(id, image_size.to_array())
					.uv0([0.0, 1.0])
					.uv1([1.0, 0.0])
					.build(ui);
			}
		}


		let input_state = engine.input.frame_state();
		{
			let orientation = Mat2x3::rotate(-player_orientation);
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
				player_orientation += rot_speed;
			}

			if input_state.active(actions.right) {
				player_orientation -= rot_speed;
			}

			if input_state.active(actions.toggle_debug) {
				debug_active = !debug_active;
			}
		}

		// Update
		{
			let pickup_distance = 1.0;

			if !player_has_sword && (player_position - sword_pos).length() < pickup_distance {
				player_has_sword = true;
			}

			if !player_has_potion && (player_position - potion_pos).length() < pickup_distance {
				player_has_potion = true;
			}
		}

		// Build meshes
		{
			mesh_data.clear();

			if !player_has_sword {
				let sword_world = sword_pos.to_x0y();

				// Drop shadow
				let surface = gfx::BuilderSurface::from_orthogonal(gfx::OrthogonalOrientation::PositiveY)
					.with_origin(sword_world + Vec3::from_y(0.05));

				build_quad_on_surface(&mut mesh_data, surface,
					Vec2i::new(16, 15*16), Vec2i::splat(8), 1.0, Vec2::zero(), Color::grey(0.1));


				// Sword
				let surface = gfx::BuilderSurface::from_quat(Quat::from_yaw(time))
					.with_origin(sword_world + Vec3::from_y(0.2 + 0.15 * time.sin()));

				build_quad_on_surface(&mut mesh_data, surface,
					Vec2i::new(0, 15*16), Vec2i::new(15, 16), 1.0, Vec2::from_y(-0.5), Color::grey(1.0));
			}


			if !player_has_potion {
				let potion_world = potion_pos.to_x0y();

				// Drop shadow
				let surface = gfx::BuilderSurface::from_orthogonal(gfx::OrthogonalOrientation::PositiveY)
					.with_origin(potion_world + Vec3::from_y(0.05));

				build_quad_on_surface(&mut mesh_data, surface,
					Vec2i::new(16, 15*16), Vec2i::splat(8), 1.0, Vec2::zero(), Color::grey(0.1));

				// Potion
				let surface = gfx::BuilderSurface::from_quat(Quat::from_yaw(player_orientation))
					.with_origin(potion_world + Vec3::from_y(0.2 + 0.15 * time.sin()));

				build_quad_on_surface(&mut mesh_data, surface,
					Vec2i::new(16 + 8, 15*16 + 8), Vec2i::new(7, 7), 1.0, Vec2::from_y(-0.5), Color::grey(1.0));
			}

			dynamic_mesh.upload(&mesh_data);
		}

		{
			mesh_data.clear();

			const UI_SCALE: f32 = 1.0 / 4.0;
			let screen_left = -engine.gfx.aspect();
			let screen_right = engine.gfx.aspect();
			let icon_width = PIXEL_SIZE * UI_SCALE * 8.0;

			// Hearts
			for i in 0..3 {
				let surface = gfx::BuilderSurface::from_orthogonal(gfx::OrthogonalOrientation::PositiveZ)
					.with_origin(Vec3::new(screen_left + (i as f32) * icon_width + 0.05, 0.95, 0.0));

				build_quad_on_surface(&mut mesh_data, surface,
					Vec2i::new(16, 15*16 + 8), Vec2i::splat(8), UI_SCALE, Vec2::new(-0.5, 0.5), Color::grey(1.0));
			}

			if player_has_sword {
				// Sword
				let surface = gfx::BuilderSurface::from_orthogonal(gfx::OrthogonalOrientation::PositiveZ)
					.with_origin(Vec3::new(screen_right - 0.05, 0.95, 0.0));

				build_quad_on_surface(&mut mesh_data, surface,
					Vec2i::new(0, 15*16), Vec2i::new(15, 16), UI_SCALE, Vec2::splat(0.5), Color::grey(1.0));
			}

			if player_has_potion {
				// Potion
				let surface = gfx::BuilderSurface::from_orthogonal(gfx::OrthogonalOrientation::PositiveZ)
					.with_origin(Vec3::new(screen_left + 0.05, -0.95, 0.0));

				build_quad_on_surface(&mut mesh_data, surface,
					Vec2i::new(16 + 8, 15*16 + 8), Vec2i::new(7, 7), UI_SCALE, Vec2::splat(-0.5), Color::grey(1.0));
			}

			// Hand
			let surface = gfx::BuilderSurface::from_orthogonal(gfx::OrthogonalOrientation::PositiveZ)
				.with_origin(Vec3::new(0.75, -1.0, 0.0));

			let frame = (time * 2.0) as i32 % 2;

			build_quad_on_surface(&mut mesh_data, surface,
				Vec2i::new(32 + frame*16, 15*16), Vec2i::splat(16), 1.0, Vec2::from_y(-0.5), Color::grey(1.0));

			ui_mesh.upload(&mesh_data);
		}


		let projection_view = {
			let camera_orientation = Mat4::rotate_y(-player_orientation);
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


		let projection_view = Mat4::ortho_aspect(1.0, engine.gfx.aspect(), -1.0, 1.0);

		std_uniform_buffer_ui.upload_single(&shaders::StdUniforms {
			projection_view,
			projection_view_inverse: projection_view.inverse(),
			ui_projection_view: Mat4::identity(),
		});

		let mut gfx = engine.gfx.draw_context();
		gfx.set_clear_color(Color::grey(0.01));
		gfx.clear(gfx::ClearMode::ALL);
		gfx.set_backface_culling(false);

		gfx.bind_texture(0, texture);
		gfx.bind_uniform_buffer(0, std_uniform_buffer);
		gfx.bind_shader(color_shader);

		scene_mesh.draw(&mut gfx, gfx::DrawMode::Triangles);
		dynamic_mesh.draw(&mut gfx, gfx::DrawMode::Triangles);

		gfx.clear(gfx::ClearMode::DEPTH);

		gfx.bind_uniform_buffer(0, std_uniform_buffer_ui);
		ui_mesh.draw(&mut gfx, gfx::DrawMode::Triangles);

		engine = next_frame(engine).await;
	}

	Ok(())
}


fn build_map() -> gfx::MeshData<TexturedVertex> {
	let mut mesh_data = gfx::MeshData::new();

	build_room(&mut mesh_data, Vec2i::zero());
	build_room(&mut mesh_data, Vec2i::new(0, -1));
	build_room(&mut mesh_data, Vec2i::new(0, -2));
	build_room(&mut mesh_data, Vec2i::new(1, -2));

	mesh_data
}


const ROOM_SIZE: f32 = 4.0;
const ROOM_HEIGHT: f32 = 2.0;

fn build_room(md: &mut gfx::MeshData<TexturedVertex>, location: Vec2i) {
	use gfx::OrthogonalOrientation;

	let origin = location.to_vec2() * ROOM_SIZE;
	let origin = origin.to_x0y();

	build_quad_on_surface(md, OrthogonalOrientation::PositiveY.to_surface_with_origin(origin + Vec3::from_y(0.0)),
		Vec2i::new(15*16, 0), Vec2i::splat(16), 4.0, Vec2::zero(), Color::grey(0.04));

	build_quad_on_surface(md, OrthogonalOrientation::NegativeY.to_surface_with_origin(origin + Vec3::from_y(ROOM_HEIGHT)),
		Vec2i::new(15*16, 0), Vec2i::splat(16), 4.0, Vec2::zero(), Color::grey(0.04));
}


use std::path::Path;

fn load_texture(gfx: &mut gfx::ResourceContext<'_>, path: impl AsRef<Path>) -> Result<(gfx::TextureKey, Vec2i), Box<dyn Error>> {
	let image = image::open(path)?.flipv().into_rgba8().into_flat_samples();
	let image_size = Vec2i::new(image.layout.width as i32, image.layout.height as i32);
	let texture_format = gfx::TextureFormat::srgba();

	let texture = gfx.new_texture(image_size, texture_format);

	{
		let mut texture = gfx.resources.textures.get_mut(texture);
		texture.upload_rgba8_raw(&image.samples);
	}

	Ok((texture, image_size))
}



// struct SpriteDef {
// 	uv_pixel: Vec2i,
// }


const PIXEL_SIZE: f32 = 1.0 / 16.0;

fn build_quad_on_surface(md: &mut gfx::MeshData<TexturedVertex>, surface: impl Into<gfx::BuilderSurface>,
	uv_start: Vec2i, uv_size: Vec2i, scale_factor: f32, anchor: Vec2, color: Color)
{
	let vertices = [
		Vec2::new(0.0, 0.0),
		Vec2::new(1.0, 0.0),
		Vec2::new(1.0, 1.0),
		Vec2::new(0.0, 1.0),
	];

	let surface = surface.into().to_mat3();
	let world_size = uv_size.to_vec2() * PIXEL_SIZE * scale_factor;

	let uv_start = uv_start.to_vec2() / Vec2::splat(256.0);
	let uv_size = uv_size.to_vec2() / Vec2::splat(256.0);

	let vertices = vertices.into_iter()
		.map(|uv| {
			let v2 = (uv - Vec2::splat(0.5) - anchor) * world_size;
			let uv = uv * uv_size + uv_start;

			TexturedVertex {
				pos: surface * v2.extend(1.0),
				color: color.into(),
				uv,
			}
		});

	md.extend(vertices, gfx::util::iter_fan_indices(4));
}




use gfx::vertex::*;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TexturedVertex {
	pub pos: Vec3,
	pub color: Vec4,
	pub uv: Vec2,
}


impl Vertex for TexturedVertex {
	fn descriptor() -> Descriptor {
		static VERTEX_ATTRIBUTES: &'static [Attribute] = &[
			Attribute::new(0*4, AttributeType::Vec3),
			Attribute::new(3*4, AttributeType::Vec4),
			Attribute::new(7*4, AttributeType::Vec2),
		];

		Descriptor {
			attributes: VERTEX_ATTRIBUTES,
			size_bytes: std::mem::size_of::<Self>() as u32,
		}
	}
}
