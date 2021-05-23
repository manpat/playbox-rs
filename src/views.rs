pub mod perf;
pub mod cube;

pub use perf::*;
pub use cube::*;

use toybox::prelude::*;
use toybox::perf::Instrumenter;


pub struct ViewContext<'engine> {
	pub gl: &'engine gl::Context,
	pub perf: &'engine mut Instrumenter,
}

impl<'engine> ViewContext<'engine> {
	pub fn new(gl: &'engine gl::Context, perf: &'engine mut Instrumenter) -> ViewContext<'engine> {
		ViewContext {
			gl,
			perf,
		}
	}
}
