use toybox::prelude::*;
use gfx::vertex::{ColorVertex, ColorVertex2D};

pub struct Mesh<V: gfx::Vertex> {
	pub vao: gfx::Vao,
	pub vertex_buffer: gfx::Buffer<V>,
	pub index_buffer: gfx::Buffer<u16>,
}

impl<V: gfx::Vertex> Mesh<V> {
	pub fn new(gfx: &gfx::Context) -> Self {
		let vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer();
		let index_buffer = gfx.new_buffer();

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		Mesh {
			vao,
			vertex_buffer,
			index_buffer,
		}
	}

	pub fn draw(&self, gfx: &gfx::Context, draw_mode: gfx::DrawMode) {
		gfx.bind_vao(self.vao);
		gfx.draw_indexed(draw_mode, self.index_buffer.len());
	}

	pub fn draw_instanced(&self, gfx: &gfx::Context, draw_mode: gfx::DrawMode, num_instances: u32) {
		gfx.bind_vao(self.vao);
		gfx.draw_instances_indexed(draw_mode, self.index_buffer.len(), num_instances);
	}

	pub fn upload(&mut self, mesh_data: &MeshData<V>) {
		self.vertex_buffer.upload(&mesh_data.vertices, gfx::BufferUsage::Dynamic);
		self.index_buffer.upload(&mesh_data.indices, gfx::BufferUsage::Dynamic);
	}

	pub fn upload_separate(&mut self, vertices: &[V], indices: &[u16]) {
		self.vertex_buffer.upload(vertices, gfx::BufferUsage::Dynamic);
		self.index_buffer.upload(indices, gfx::BufferUsage::Dynamic);
	}
}


// MeshData
pub struct MeshData<V: gfx::Vertex> {
	pub vertices: Vec<V>,
	pub indices: Vec<u16>,
}

impl<V: gfx::Vertex> MeshData<V> {
	pub fn new() -> Self {
		MeshData {
			vertices: Vec::new(),
			indices: Vec::new(),
		}
	}

	pub fn extend(&mut self, vs: impl IntoIterator<Item=V>, is: impl IntoIterator<Item=u16>) {
		let index_start = self.vertices.len() as u16;
		self.vertices.extend(vs);
		self.indices.extend(is.into_iter().map(|idx| index_start + idx));
	}
}


// Traits
pub trait PolyBuilder2D {
	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>);
}

pub trait PolyBuilder3D {
	// type OnPlane : PolyBuilder2D;

	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>);
	// fn on_plane(&mut self, uvw: Mat2x3) -> Self::OnPlane;
}

pub trait ColoredPolyBuilder {
	fn set_color(&mut self, color: impl Into<Color>);
}


// Generic Plane Builder
// pub struct GenericMeshBuilderOnPlane<'mb, MB: PolyBuilder3D> {
// 	builder_3d: &'mb mut MB,
// 	uvw: Mat2x3,
// }

// impl<MB: PolyBuilder3D> PolyBuilder2D for GenericMeshBuilderOnPlane<'_, MB> {
// 	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>) {
// 		let vertices_3d = vs.into_iter().map(|v2| self.uvw * v2);
// 		self.builder_3d.extend_3d(vertices_3d, is);
// 	}
// }

// impl<B> ColoredPolyBuilder for GenericMeshBuilderOnPlane<'_, B>
// 	where B: PolyBuilder3D + ColoredPolyBuilder
// {
// 	fn set_color(&mut self, color: impl Into<Color>) {
// 		self.builder_3d.set_color(color);
// 	}
// }


// ColorVertex
pub struct ColorMeshBuilder<'md> {
	data: &'md mut MeshData<ColorVertex>,
	color: Color,
}

impl<'md> PolyBuilder3D for ColorMeshBuilder<'md> {
	// type OnPlane = GenericMeshBuilderOnPlane<'md, Self>;

	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>) {
		let color = self.color.into();
		self.data.extend(vs.into_iter().map(|v| ColorVertex::new(v, color)), is);
	}

	// fn on_plane(&mut self, uvw: Mat2x3) -> Self::OnPlane {
	// 	Self::OnPlane {
	// 		builder_3d: self,
	// 		uvw,
	// 	}
	// }
}

impl ColoredPolyBuilder for ColorMeshBuilder<'_> {
	fn set_color(&mut self, color: impl Into<Color>) {
		self.color = color.into();
	}
}



// ColorVertex2D
// pub struct Color2DMeshBuilder<'md> {
// 	data: &'md mut MeshData<ColorVertex2D>,
// 	color: Color,
// }

// impl<'md> PolyBuilder2D for Color2DMeshBuilder<'md> {
// 	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>) {
// 		self.data.extend(vs.into_iter().map(|v| ColorVertex2D::new(v, self.color)), is);
// 	}
// }

// impl<'md> ColoredPolyBuilder for Color2DMeshBuilder<'md> {
// 	fn set_color(&mut self, color: impl Into<Color>) {
// 		self.color = color.into();
// 	}
// }


// Generic
// pub struct GenericMeshBuilder<'md, V: gfx::Vertex, F: FnMut(Vec3) -> V + 'static> {
// 	data: &'md mut MeshData<V>,
// 	new_vertex: F,
// }

// impl<'md, V: gfx::Vertex, F: FnMut(Vec3) -> V + 'static> PolyBuilder3D for GenericMeshBuilder<'md, V, F> {
// 	type OnPlane = GenericMeshBuilderOnPlane<'md, Self>;

// 	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>) {
// 		self.data.extend(vs.into_iter().map(self.new_vertex), is);
// 	}

// 	fn on_plane(&mut self, uvw: Mat2x3) -> Self::OnPlane {
// 		Self::OnPlane {
// 			builder_3d: self,
// 			uvw,
// 		}
// 	}
// }



// Geo

// pub trait PolyBuilder2DExt {
// 	fn circle(&mut self, center: Vec2, radius: f32);
// 	fn quad(&mut self, center: Vec2, size: Vec2);
// 	fn polygon(&mut self, center: Vec2, num_edges: usize, radius: f32, rotation: f32);
// 	fn wedge(&mut self, center: Vec2, num_edges: usize, radius: f32, rotation: f32);
// }


// impl<Builder> PolyBuilder2DExt for Builder
// 	where Builder: PolyBuilder2D
// {
// 	fn circle(&mut self, center: Vec2, radius: f32) {

// 	}

// 	fn quad(&mut self, center: Vec2, size: Vec2) {

// 	}

// 	fn polygon(&mut self, center: Vec2, num_edges: usize, radius: f32, rotation: f32) {

// 	}

// 	fn wedge(&mut self, center: Vec2, num_edges: usize, radius: f32, rotation: f32) {

// 	}
// }


// pub trait PolyBuilder3DExt {
// 	fn cuboid(&mut self, center: Vec3, size: Vec3);
// 	fn cylinder(&mut self, center: Vec3, size: Vec3);
// }


// impl<Builder> PolyBuilder3DExt for Builder
// 	where Builder: PolyBuilder2D
// {
// 	fn cuboid(&mut self, center: Vec3, size: Vec3) {

// 	}

// 	fn cylinder(&mut self, center: Vec3, size: Vec3) {

// 	}
// }

