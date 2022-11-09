use toybox::prelude::*;
use gfx::vertex::ColorVertex;
use gfx::mesh::{Mesh, MeshData};

use crate::vignette::platformer3d::model;

mod gem;

pub struct SceneView {
	color_shader: gfx::Shader,
	textured_shader: gfx::Shader,

	color_mesh: Mesh<ColorVertex>,
	textured_mesh: Mesh<TexturedVertex>,

	texture: gfx::TextureKey,

	gem_view: gem::GemView,
}

impl SceneView {
	pub fn new(gfx: &mut gfx::ResourceContext<'_>, scene: &model::Scene) -> Result<SceneView, Box<dyn Error>> {
		let color_shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;

		let textured_shader = gfx.new_simple_shader(
			crate::shaders::TEX_3D_VERT,
			crate::shaders::TEXTURED_FRAG,
		)?;

		let mut color_mesh_data = MeshData::new();
		let mut textured_mesh_data = MeshData::new();
		let main_scene = scene.main_scene();

		for entity in main_scene.entities().filter(|e| !e.name.contains('_') || e.name.starts_with("SOUND_")) {
			build_entity_transformed(&mut color_mesh_data, &mut textured_mesh_data, entity, entity.transform());
		}

		let mut color_mesh = Mesh::new(gfx);
		color_mesh.upload(&color_mesh_data);

		let mut textured_mesh = Mesh::new(gfx);
		textured_mesh.upload(&textured_mesh_data);

		let image = image::open("assets/mytex.png")?.flipv().into_rgba8().into_flat_samples();
		let image_size = Vec2i::new(image.layout.width as i32, image.layout.height as i32);
		let texture_format = gfx::TextureFormat::srgba();

		let texture = gfx.new_texture(image_size, texture_format);

		{
			let mut texture = gfx.resources.textures.get_mut(texture);
			texture.upload_rgba8_raw(&image.samples);
		}

		Ok(SceneView {
			color_shader,
			textured_shader,

			color_mesh,
			textured_mesh,

			texture,

			gem_view: gem::GemView::new(gfx, scene)?,
		})
	}

	#[instrument(skip_all)]
	pub fn update(&mut self, scene: &model::Scene, blob_shadows: &mut model::BlobShadowModel) {
		self.gem_view.update(scene, blob_shadows);
	}

	#[instrument(skip_all)]
	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("scene");

		ctx.gfx.bind_shader(self.color_shader);
		self.color_mesh.draw(&mut ctx.gfx, gfx::DrawMode::Triangles);

		ctx.gfx.set_backface_culling(false);
		ctx.gfx.bind_shader(self.textured_shader);
		ctx.gfx.bind_texture(0, self.texture);
		self.textured_mesh.draw(&mut ctx.gfx, gfx::DrawMode::Triangles);

		self.gem_view.draw(&mut ctx.gfx);
	}
}



fn build_entity_transformed(color_mesh_data: &mut MeshData<ColorVertex>,
	textured_mesh_data: &mut MeshData<TexturedVertex>,
	entity: toy::EntityRef<'_>, transform: Mat3x4)
{
	use itertools::Either::*;

	let ent_mesh_data = entity.mesh().unwrap();
	let indices = ent_mesh_data.indices.iter().cloned();

	let vert_color = match ent_mesh_data.color_layers.first() {
		Some(color_layer) => Left(color_layer.data.iter().copied()),
		None => Right(std::iter::repeat(Vec4::splat(1.0))),
	};

	if let Some(uv_layer) = ent_mesh_data.uv_layers.first() {
		let ent_vertices = ent_mesh_data.positions.iter()
			.zip(uv_layer.data.iter().zip(vert_color))
			.map(move |(&pos, (&uv, color))| {
				let pos = transform * pos;
				TexturedVertex {
					pos,
					color,
					uv
				}
			});

		textured_mesh_data.extend(ent_vertices, indices);
		return
	}

	let ent_vertices = ent_mesh_data.positions.iter()
		.zip(vert_color)
		.map(move |(&p, col)| {
			let p = transform * p;
			ColorVertex::new(p, col)
		});

	color_mesh_data.extend(ent_vertices, indices);
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
