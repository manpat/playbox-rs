use toybox::prelude::*;
use crate::executor::{start_loop, next_frame};
use crate::shaders;


mod sprite_builder;

use sprite_builder::*;




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


	let texture = load_texture(&mut resource_ctx, "assets/dungeon.png")?;



	let mut player_position = Vec2::zero();
	let mut player_orientation = 0.0;

	let mut player_has_sword = false;
	let mut player_has_potion = false;

	let sword_pos = Vec2::new(0.0, -5.0);
	let potion_pos = Vec2::new(2.0, -8.0);

	let mut time = 0.0f32;


	let sword_sprite = Sprite::new(Vec2i::new(0, 15*16), Vec2i::new(15, 16)).with_anchor(Anchor::S);
	let potion_sprite = Sprite::new(Vec2i::new(16 + 8, 15*16 + 8), Vec2i::splat(7));
	let drop_glow_sprite = Sprite::new(Vec2i::new(16, 15*16), Vec2i::splat(8));

	let heart_sprite = Sprite::new(Vec2i::new(16, 15*16 + 8), Vec2i::splat(8));
	let hand_sprites = [
		Sprite::new(Vec2i::new(32 + 0*16, 15*16), Vec2i::splat(16)),
		Sprite::new(Vec2i::new(32 + 1*16, 15*16), Vec2i::splat(16)),
	];


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

			let mut mb = SpriteBuilder::new(&mut mesh_data).with_yaw(player_orientation);

			if !player_has_sword {
				let sword_world = sword_pos.to_x0y();

				// Drop shadow
				let surface = gfx::OrthogonalOrientation::PositiveY
					.to_surface_with_origin(sword_world + Vec3::from_y(0.05));

				mb.tint_color = Color::grey(0.1);
				mb.build_on_surface(&drop_glow_sprite, surface);


				// Sword
				let surface = gfx::BuilderSurface::from_quat(Quat::from_yaw(time))
					.with_origin(sword_world + Vec3::from_y(0.2 + 0.15 * time.sin()));

				mb.tint_color = Color::white();
				mb.build_on_surface(&sword_sprite, surface);
			}


			if !player_has_potion {
				let potion_world = potion_pos.to_x0y();

				// Drop shadow
				let surface = gfx::OrthogonalOrientation::PositiveY
					.to_surface_with_origin(potion_world + Vec3::from_y(0.05));

				mb.tint_color = Color::grey(0.1);
				mb.build_on_surface(&drop_glow_sprite, surface);

				// Potion
				mb.tint_color = Color::white();
				mb.build(&potion_sprite.with_anchor(Anchor::S), potion_world + Vec3::from_y(0.2 + 0.15 * time.sin()));
			}

			dynamic_mesh.upload(&mesh_data);
		}

		{
			mesh_data.clear();

			const UI_SCALE: f32 = 1.0 / 4.0;
			let icon_width = PIXEL_WORLD_SIZE * UI_SCALE * 8.0;


			let mut mb = SpriteBuilder::new(&mut mesh_data).for_screen(engine.gfx.aspect());
			mb.scale_factor = UI_SCALE;
			mb.tint_color = Color::white();
			mb.margin = 0.05;

			// Hearts
			for i in 0..3 {
				mb.build_with_anchor(&heart_sprite.with_anchor(Anchor::NW), Anchor::NW,
					Vec2::from_x((i as f32) * icon_width));
			}

			if player_has_sword {
				mb.build_with_anchor(&sword_sprite.with_anchor(Anchor::NE), Anchor::NE, Vec2::zero());
			}

			if player_has_potion {
				mb.build_with_anchor(&potion_sprite.with_anchor(Anchor::SW), Anchor::SW, Vec2::zero());
			}

			// Hand
			let frame = (time * 2.0) as usize % 2;

			let hand_sprite = hand_sprites[frame];
			mb.scale_factor = 1.0;
			mb.margin = 0.0;
			mb.build_with_anchor(&hand_sprite.with_anchor(Anchor::S), Anchor::S, Vec2::from_x(0.5));

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
			projection_view_inverse: projection_view.inverse()
		});


		let projection_view = Mat4::ortho_aspect(1.0, engine.gfx.aspect(), -1.0, 1.0);

		std_uniform_buffer_ui.upload_single(&shaders::StdUniforms {
			projection_view,
			projection_view_inverse: projection_view.inverse()
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
	let mut mb = SpriteBuilder::new(&mut mesh_data);

	build_room(&mut mb, Vec2i::zero(), [true, false, false, false]);
	build_room(&mut mb, Vec2i::new(0, -1), [true, false, true, false]);
	build_room(&mut mb, Vec2i::new(0, -2), [false, true, true, false]);
	build_room(&mut mb, Vec2i::new(1, -2), [false, false, false, true]);

	mesh_data
}


const ROOM_SIZE: f32 = 4.0;
const ROOM_HEIGHT: f32 = 2.0;

fn build_room(mb: &mut SpriteBuilder<'_>, location: Vec2i, adjacency: [bool; 4]) {
	use gfx::OrthogonalOrientation;

	let origin = location.to_vec2() * ROOM_SIZE;
	let origin = origin.to_x0y();

	let floor_sprite = Sprite::new(Vec2i::new(4*16, 0), Vec2i::splat(32));
	let ceil_sprite = Sprite::new(Vec2i::new(6*16, 0), Vec2i::splat(32));
	let wall_sprite = Sprite::new(Vec2i::new(2*16, 0), Vec2i::new(32, 16)).with_anchor(Anchor::S);

	mb.tint_color = Color::grey(0.2);
	mb.scale_factor = 2.0;
	mb.build_on_surface(&floor_sprite, OrthogonalOrientation::PositiveY.to_surface_with_origin(origin + Vec3::from_y(0.0)));
	mb.build_on_surface(&ceil_sprite, OrthogonalOrientation::NegativeY.to_surface_with_origin(origin + Vec3::from_y(ROOM_HEIGHT)));

	if !adjacency[0] {
		mb.build_on_surface(&wall_sprite, OrthogonalOrientation::PositiveZ.to_surface_with_origin(origin + Vec3::from_z(-ROOM_SIZE / 2.0)));
	}

	if !adjacency[1] {
		mb.build_on_surface(&wall_sprite, OrthogonalOrientation::NegativeX.to_surface_with_origin(origin + Vec3::from_x(ROOM_SIZE / 2.0)));
	}

	if !adjacency[2] {
		mb.build_on_surface(&wall_sprite, OrthogonalOrientation::NegativeZ.to_surface_with_origin(origin + Vec3::from_z(ROOM_SIZE / 2.0)));
	}

	if !adjacency[3] {
		mb.build_on_surface(&wall_sprite, OrthogonalOrientation::PositiveX.to_surface_with_origin(origin + Vec3::from_x(-ROOM_SIZE / 2.0)));
	}
}



use std::path::Path;

fn load_texture(gfx: &mut gfx::ResourceContext<'_>, path: impl AsRef<Path>) -> Result<gfx::TextureKey, Box<dyn Error>> {
	let image = image::open(path)?.flipv().into_rgba8().into_flat_samples();
	let image_size = Vec2i::new(image.layout.width as i32, image.layout.height as i32);

	assert!(image_size == Vec2i::splat(TEXTURE_SIZE as i32));

	let texture_format = gfx::TextureFormat::srgba();
	let texture = gfx.new_texture(image_size, texture_format);

	{
		let mut texture = gfx.resources.textures.get_mut(texture);
		texture.upload_rgba8_raw(&image.samples);
	}

	Ok(texture)
}


