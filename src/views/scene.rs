use toybox::prelude::*;
use gfx::vertex::ColorVertex;

use crate::model;

pub struct SceneView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	num_elements: u32,

	gem_view: GemView,
}

impl SceneView {
	pub fn new(gfx: &gfx::Context, scene: &model::Scene) -> Result<SceneView, Box<dyn Error>> {
		let shader = gfx.new_shader(&[
			(gfx::raw::VERTEX_SHADER, include_str!("../shaders/color_3d.vert.glsl")),
			(gfx::raw::FRAGMENT_SHADER, include_str!("../shaders/flat_color.frag.glsl")),
		])?;

		let vao = gfx.new_vao();

		let mut vertex_buffer = gfx.new_buffer::<ColorVertex>();
		let mut index_buffer = gfx.new_buffer::<u16>();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		let mut vertices = Vec::new();
		let mut indices = Vec::new();

		let main_scene = scene.source_data.find_scene("main").unwrap();

		for entity in main_scene.entities().filter(|e| !e.name.contains('_')) {
			build_entity_transformed(&mut vertices, &mut indices, entity, entity.transform());
		}

		vertex_buffer.upload(&vertices, gfx::BufferUsage::Static);
		index_buffer.upload(&indices, gfx::BufferUsage::Static);

		Ok(SceneView {
			shader,
			vao,
			num_elements: indices.len() as u32,

			gem_view: GemView::new(gfx, scene)?,
		})
	}

	pub fn update(&mut self, scene: &model::Scene) {
		self.gem_view.update(scene);
	}

	pub fn draw(&self, ctx: &mut super::ViewContext) {
		let _section = ctx.perf.scoped_section("scene");

		ctx.gfx.bind_vao(self.vao);
		ctx.gfx.bind_shader(self.shader);
		ctx.gfx.draw_indexed(gfx::DrawMode::Triangles, self.num_elements);

		self.gem_view.draw(&ctx.gfx);
	}
}



fn build_entity_transformed(vertices: &mut Vec<ColorVertex>, indices: &mut Vec<u16>,
	entity: toy::EntityRef<'_>, transform: Mat3x4)
{
	let mesh_data = entity.mesh_data().unwrap();

	let color_data = mesh_data.color_data(None).unwrap();

	let ent_vertices = mesh_data.positions.iter()
		.zip(&color_data.data)
		.map(move |(&p, &col)| {
			let p = transform * p;
			ColorVertex::new(p, col.to_vec3())
		});

	let vertex_base = vertices.len() as u16;

	vertices.extend(ent_vertices);
	indices.extend(mesh_data.indices.iter().map(|&i| vertex_base + i));
}



struct GemViewData {
	anim_phase: f32,
}

struct GemView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	instance_buffer: gfx::Buffer<Mat3x4>,
	num_elements: u32,
	num_instances: u32,

	gem_view_data: Vec<GemViewData>,
}

impl GemView {
	fn new(gfx: &gfx::Context, scene: &model::Scene) -> Result<GemView, Box<dyn Error>> {
		let shader = gfx.new_shader(&[
			(gfx::raw::VERTEX_SHADER, include_str!("../shaders/color_3d_instanced_transform.vert.glsl")),
			(gfx::raw::FRAGMENT_SHADER, include_str!("../shaders/flat_color.frag.glsl")),
		])?;


		let gem_prototype = scene.source_data.find_entity("GEM_prototype").unwrap();

		let mut vertices = Vec::new();
		let mut indices = Vec::new();
		build_entity_transformed(&mut vertices, &mut indices, gem_prototype, Mat3x4::identity());

		let instance_buffer = gfx.new_buffer::<Mat3x4>();
		let mut vertex_buffer = gfx.new_buffer::<ColorVertex>();
		let mut index_buffer = gfx.new_buffer::<u16>();

		vertex_buffer.upload(&vertices, gfx::BufferUsage::Static);
		index_buffer.upload(&indices, gfx::BufferUsage::Static);

		let vao = gfx.new_vao();
		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		let gem_view_data = (0..scene.gems.len())
			.map(|idx| GemViewData {
				anim_phase: idx as f32 * PI / 3.0,
			})
			.collect();

		Ok(GemView {
			shader,
			vao,
			instance_buffer,
			num_elements: indices.len() as u32,
			num_instances: scene.gems.len() as u32,

			gem_view_data,
		})
	}

	fn update(&mut self, scene: &model::Scene) {
		for GemViewData{anim_phase} in self.gem_view_data.iter_mut() {
			*anim_phase += 1.0 / 60.0;
		}

		let instances: Vec<_> = scene.gems.iter().zip(&self.gem_view_data)
			.filter(|(gem, _)| gem.active)
			.map(|(gem, GemViewData{anim_phase})| {
				let pos = gem.position + Vec3::from_y(anim_phase.sin() * 0.3);
				let rot = *anim_phase;
				Mat3x4::rotate_y_translate(rot, pos)
			})
			.collect();

		self.instance_buffer.upload(&instances, gfx::BufferUsage::Dynamic);
	}

	fn draw(&self, gfx: &gfx::Context) {
		gfx.bind_vao(self.vao);
		gfx.bind_shader(self.shader);
		gfx.bind_shader_storage_buffer(0, self.instance_buffer);
		gfx.draw_instances_indexed(gfx::DrawMode::Triangles, self.num_elements, self.num_instances);
	}
}