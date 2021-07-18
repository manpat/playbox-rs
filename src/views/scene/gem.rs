use toybox::prelude::*;
use gfx::vertex::ColorVertex;

use crate::model;
use super::build_entity_transformed;

struct GemViewData {
	anim_phase: f32,
}

pub struct GemView {
	shader: gfx::Shader,
	vao: gfx::Vao,
	instance_buffer: gfx::Buffer<Mat3x4>,
	num_elements: u32,
	num_instances: u32,

	gem_view_data: Vec<GemViewData>,
}

impl GemView {
	pub fn new(gfx: &gfx::Context, scene: &model::Scene) -> Result<GemView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_INSTANCED_TRANFORM_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;


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

	pub fn update(&mut self, scene: &model::Scene) {
		use model::scene::GemState;

		for GemViewData{anim_phase} in self.gem_view_data.iter_mut() {
			*anim_phase += 1.0 / 60.0;
		}

		let instances: Vec<_> = scene.gems.iter().zip(&self.gem_view_data)
			.filter_map(|(gem, GemViewData{anim_phase})| {
				match gem.state {
					GemState::Idle => {
						let pos = gem.position + Vec3::from_y((anim_phase*2.0).sin() * 0.4);
						let rot = *anim_phase;
						Some(Mat3x4::rotate_y_translate(rot, pos))
					}

					GemState::Collecting(t) => {
						let float_away = t.ease_back_in(0.0, 6.0);
						let pos = gem.position + Vec3::from_y((anim_phase*2.0).sin() * 0.4 + float_away);
						let rot = *anim_phase;
						Some(Mat3x4::rotate_y_translate(rot, pos))
					}

					GemState::Collected => None,
				}
			})
			.collect();

		self.num_instances = instances.len() as u32;

		if !instances.is_empty() {
			self.instance_buffer.upload(&instances, gfx::BufferUsage::Dynamic);
		}
	}

	pub fn draw(&self, gfx: &gfx::Context) {
		if self.num_instances == 0 {
			return
		}

		gfx.bind_vao(self.vao);
		gfx.bind_shader(self.shader);
		gfx.bind_shader_storage_buffer(0, self.instance_buffer);
		gfx.draw_instances_indexed(gfx::DrawMode::Triangles, self.num_elements, self.num_instances);
	}
}