pub mod player;
pub mod scene;
pub mod debug;
pub mod blob_shadow;
pub mod mesh_builder_test;
pub mod gbuffer_particles;

pub use player::*;
pub use scene::*;
pub use debug::*;
pub use blob_shadow::*;
pub use mesh_builder_test::*;
pub use gbuffer_particles::*;

use toybox::prelude::*;
use toybox::perf::Instrumenter;


pub struct ViewContext<'engine> {
	pub gfx: gfx::RenderState<'engine>,
	pub resources: &'engine gfx::Resources,
	pub perf: &'engine mut Instrumenter,
	pub imgui: &'engine imgui::Ui<'static>,
}

impl<'engine> ViewContext<'engine> {
	pub fn new(gfx: gfx::RenderState<'engine>, perf: &'engine mut Instrumenter, imgui: &'engine imgui::Ui<'static>) -> ViewContext<'engine> {
		let resources = gfx.resources();
		ViewContext {
			gfx,
			resources,
			perf,
			imgui,
		}
	}
}
