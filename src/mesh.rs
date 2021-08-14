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

	pub fn clear(&mut self) {
		self.vertices.clear();
		self.indices.clear();
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
	fn build(&mut self, geom: impl BuildableGeometry2D) where Self: Sized { geom.build(self) }

	fn extend_2d_fan(&mut self, num_vertices: u32, vs: impl IntoIterator<Item=Vec2>) {
		if num_vertices < 3 {
			return
		}

		self.extend_2d(vs, (0..num_vertices as u16-2).flat_map(|i| [0, i+1, i+2]));
	}
}

pub trait PolyBuilder3D {
	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>);
	fn build(&mut self, geom: impl BuildableGeometry3D) where Self: Sized { geom.build(self) }

	fn extend_3d_fan(&mut self, num_vertices: u32, vs: impl IntoIterator<Item=Vec3>) {
		if num_vertices < 3 {
			return
		}

		self.extend_3d(vs, (0..num_vertices as u16-2).flat_map(|i| [0, i+1, i+2]));
	}
}

pub trait ColoredPolyBuilder {
	fn set_color(&mut self, color: impl Into<Color>);
}


// Generic Plane Builder
pub struct GenericMeshBuilderOnPlane<'mb, MB: PolyBuilder3D> {
	builder_3d: &'mb mut MB,
	uvw: Mat3,
}


impl<'mb, MB: PolyBuilder3D> GenericMeshBuilderOnPlane<'mb, MB> {
	pub fn new(builder_3d: &'mb mut MB, uvw: Mat3) -> Self {
		GenericMeshBuilderOnPlane {
			builder_3d,
			uvw,
		}
	}
}

impl<MB: PolyBuilder3D> PolyBuilder2D for GenericMeshBuilderOnPlane<'_, MB> {
	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>) {
		let uvw = self.uvw;
		
		let vertices_3d = vs.into_iter().map(move |v2| {
			uvw * v2.extend(1.0)
		});

		self.builder_3d.extend_3d(vertices_3d, is);
	}
}

impl<MB> ColoredPolyBuilder for GenericMeshBuilderOnPlane<'_, MB>
	where MB: PolyBuilder3D + ColoredPolyBuilder
{
	fn set_color(&mut self, color: impl Into<Color>) {
		self.builder_3d.set_color(color);
	}
}


// ColorVertex
pub struct ColorMeshBuilder<'md> {
	data: &'md mut MeshData<ColorVertex>,
	color: Color,
}

impl<'md> ColorMeshBuilder<'md> {
	pub fn new(data: &'md mut MeshData<ColorVertex>) -> Self {
		ColorMeshBuilder {
			data,
			color: Color::white(),
		}
	}

	pub fn set_color(&mut self, color: impl Into<Color>) {
		self.color = color.into();
	}

	pub fn on_plane(&mut self, uvw: Mat3) -> GenericMeshBuilderOnPlane<'_, Self> {
		GenericMeshBuilderOnPlane::new(self, uvw)
	}
}

impl PolyBuilder3D for ColorMeshBuilder<'_> {
	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>) {
		let color = self.color.into();
		self.data.extend(vs.into_iter().map(|v| ColorVertex::new(v, color)), is);
	}
}

impl ColoredPolyBuilder for ColorMeshBuilder<'_> {
	fn set_color(&mut self, color: impl Into<Color>) {
		self.set_color(color);
	}
}



// ColorVertex2D
pub struct ColorMeshBuilder2D<'md> {
	data: &'md mut MeshData<ColorVertex2D>,
	color: Color,
}

impl<'md> ColorMeshBuilder2D<'md> {
	pub fn new(data: &'md mut MeshData<ColorVertex2D>) -> Self {
		ColorMeshBuilder2D {
			data,
			color: Color::white(),
		}
	}

	pub fn set_color(&mut self, color: impl Into<Color>) {
		self.color = color.into();
	}
}

impl PolyBuilder2D for ColorMeshBuilder2D<'_> {
	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>) {
		let color = self.color.into();
		self.data.extend(vs.into_iter().map(|v| ColorVertex2D::new(v, color)), is);
	}
}

impl ColoredPolyBuilder for ColorMeshBuilder2D<'_> {
	fn set_color(&mut self, color: impl Into<Color>) {
		self.set_color(color);
	}
}


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


pub trait BuildableGeometry3D {
	fn build<MB: PolyBuilder3D>(&self, mb: &mut MB);
}

pub trait BuildableGeometry2D {
	fn build<MB: PolyBuilder2D>(&self, mb: &mut MB);
}


impl<G: BuildableGeometry2D> BuildableGeometry2D for &G {
	fn build<MB: PolyBuilder2D>(&self, mb: &mut MB) {
		(*self).build(mb);
	}
}

impl<G: BuildableGeometry3D> BuildableGeometry3D for &G {
	fn build<MB: PolyBuilder3D>(&self, mb: &mut MB) {
		(*self).build(mb);
	}
}



pub mod geom {
	use super::*;

	// pub struct Cuboid {
	// 	basis: Mat3x4,
	// }

	// impl Cuboid {
	// 	pub fn unit() -> Cuboid {
	// 		Cuboid {
	// 			basis: Mat3x4::identity(),
	// 		}
	// 	}


	// 	pub fn build<MB: PolyBuilder3D>(&self, mb: &mut MB) {
	// 		let verts = [

	// 		];

	// 		let indices = [
			
	// 		];

	// 		mb.extend_3d(verts, indices);
	// 	}
	// }


	pub struct Quad {
		basis: Mat2x3,
	}

	impl Quad {
		pub fn from_matrix(basis: Mat2x3) -> Quad {
			Quad {basis}
		}

		pub fn unit() -> Quad {
			Quad::from_matrix(Mat2x3::identity())
		}
	}

	impl BuildableGeometry2D for Quad {
		fn build<MB: PolyBuilder2D>(&self, mb: &mut MB) {
			let [ux, uy, translation] = self.basis.columns();
			let (hx, hy) = (ux/2.0, uy/2.0);

			mb.extend_2d_fan(4, [
				translation - hx - hy,
				translation - hx + hy,
				translation + hx + hy,
				translation + hx - hy,
			]);
		}
	}


	pub struct Polygon {
		basis: Mat2x3,
		num_faces: u32,
	}

	impl Polygon {
		pub fn from_matrix(num_faces: u32, basis: Mat2x3) -> Polygon {
			Polygon {basis, num_faces}
		}

		pub fn unit(num_faces: u32) -> Polygon {
			Polygon::from_matrix(num_faces, Mat2x3::identity())
		}
	}

	impl BuildableGeometry2D for Polygon {
		fn build<MB: PolyBuilder2D>(&self, mb: &mut MB) {
			if self.num_faces < 3 {
				return
			}

			let [ux, uy, translation] = self.basis.columns();
			let uxy = Mat2::from_columns([ux/2.0, uy/2.0]);

			let angle_increment = TAU / (self.num_faces as f32);
			let vertices = (0..self.num_faces)
				.map(|i| {
					let angle = angle_increment * i as f32;
					translation + uxy * Vec2::from_angle(angle)
				});

			mb.extend_2d_fan(self.num_faces, vertices);
		}
	}



	pub struct Tetrahedron {
		basis: Mat3x4,
	}

	impl Tetrahedron {
		pub fn from_matrix(basis: Mat3x4) -> Tetrahedron {
			Tetrahedron {basis}
		}

		pub fn unit() -> Tetrahedron {
			Tetrahedron::from_matrix(Mat3x4::identity())
		}
	}

	impl BuildableGeometry3D for Tetrahedron {
		fn build<MB: PolyBuilder3D>(&self, mb: &mut MB) {
			let [ux, uy, uz, translation] = self.basis.columns();

			let verts = [
				translation + ux,
				translation + ux*(TAU/3.0).cos() + uz*(TAU/3.0).sin(),
				translation + ux*(TAU/3.0).cos() - uz*(TAU/3.0).sin(),
				translation + uy,
			];

			let indices = [
				0, 1, 2,

				3, 0, 1,
				3, 1, 2,
				3, 2, 0,
			];

			mb.extend_3d(verts, indices);
		}
	}
}


