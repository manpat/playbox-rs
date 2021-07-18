pub mod perf;
pub mod player;
pub mod scene;
pub mod debug;
pub mod blob_shadow;

pub use perf::*;
pub use player::*;
pub use scene::*;
pub use debug::*;
pub use blob_shadow::*;

use toybox::prelude::*;
use toybox::perf::Instrumenter;


pub struct ViewContext<'engine> {
	pub gfx: &'engine gfx::Context,
	pub perf: &'engine mut Instrumenter,
}

impl<'engine> ViewContext<'engine> {
	pub fn new(gfx: &'engine gfx::Context, perf: &'engine mut Instrumenter) -> ViewContext<'engine> {
		ViewContext {
			gfx,
			perf,
		}
	}
}
