use toybox::prelude::*;

pub struct PerfView {
	vao: gl::Vao,
	vertex_buffer: gl::Buffer<gl::vertex::ColorVertex2D>,
}