use toybox::prelude::*;
use gfx::vertex::ColorVertex;

use crate::model;
use crate::mesh::{Mesh, MeshData};

use super::build_entity_transformed;


struct GemViewData {
	anim_phase: f32,
}

pub struct GemView {
	shader: gfx::Shader,
	mesh: Mesh<ColorVertex>,
	instance_buffer: gfx::Buffer<Mat3x4>,

	gem_view_data: Vec<GemViewData>,
}

impl GemView {
	pub fn new(gfx: &gfx::Context, scene: &model::Scene) -> Result<GemView, Box<dyn Error>> {
		let shader = gfx.new_simple_shader(
			crate::shaders::COLOR_3D_INSTANCED_TRANFORM_VERT,
			crate::shaders::FLAT_COLOR_FRAG,
		)?;


		let gem_prototype = scene.source_data.find_entity("GEM_prototype").unwrap();

		let mut mesh_data = MeshData::new();
		build_entity_transformed(&mut mesh_data.vertices, &mut mesh_data.indices, gem_prototype, Mat3x4::identity());

		let mut mesh = Mesh::new(gfx);
		mesh.upload(&mesh_data);

		let instance_buffer = gfx.new_buffer::<Mat3x4>();

		let gem_view_data = (0..scene.gems.len())
			.map(|idx| GemViewData {
				anim_phase: idx as f32 * PI / 3.0,
			})
			.collect();

		Ok(GemView {
			shader,
			mesh,
			instance_buffer,

			gem_view_data,
		})
	}

	pub fn update(&mut self, scene: &model::Scene, blob_shadows: &mut model::BlobShadowModel) {
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
						let scale = t.ease_back_in(1.0, 0.0);
						let pos = gem.position + Vec3::from_y((anim_phase*2.0).sin() * 0.4 + float_away);
						let rot = *anim_phase;
						Some(Mat3x4::rotate_y_translate(rot, pos) * Mat3x4::uniform_scale(scale))
					}

					GemState::Collected => None,
				}
			})
			.collect();

		if !instances.is_empty() {
			self.instance_buffer.upload(&instances, gfx::BufferUsage::Dynamic);
		}

		for inst in instances.iter() {
			blob_shadows.add(inst.column_w(), 2.0);
		}
	}

	pub fn draw(&self, gfx: &gfx::Context) {
		if self.instance_buffer.is_empty() {
			return
		}

		gfx.bind_shader(self.shader);
		gfx.bind_shader_storage_buffer(0, self.instance_buffer);

		self.mesh.draw_instanced(gfx, gfx::DrawMode::Triangles, self.instance_buffer.len());
	}
}