pub mod perf;
pub mod cube;
pub mod player;
pub mod scene;
pub mod debug;

pub use perf::*;
pub use cube::*;
pub use player::*;
pub use scene::*;
pub use debug::*;

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
