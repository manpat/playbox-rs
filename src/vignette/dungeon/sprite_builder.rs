use toybox::prelude::*;
use gfx::vertex::*;


pub const PIXEL_WORLD_SIZE: f32 = 1.0 / 16.0;
pub const TEXTURE_SIZE: u32 = 256;
pub const TEXEL_SIZE: f32 = 1.0 / TEXTURE_SIZE as f32;


pub struct SpriteBuilder<'md> {
	md: &'md mut gfx::MeshData<TexturedVertex>,

	pub tint_color: Color,
	pub scale_factor: f32,
}

impl<'md> SpriteBuilder<'md> {
	pub fn new(md: &'md mut gfx::MeshData<TexturedVertex>) -> Self {
		SpriteBuilder {
			md,

			tint_color: Color::white(),
			scale_factor: 1.0,
		}
	}

	pub fn for_screen(self, aspect: f32) -> ScreenSpriteBuilder<'md> {
		ScreenSpriteBuilder {
			builder: self,
			aspect,
			margin: 0.0,
		}
	}

	pub fn with_yaw(self, yaw: f32) -> SurfaceSpriteBuilder<'md> {
		SurfaceSpriteBuilder {
			builder: self,
			surface: gfx::BuilderSurface::from_quat(Quat::from_yaw(yaw)),
		}
	}

	pub fn build_on_surface(&mut self, sprite: &Sprite, surface: impl Into<gfx::BuilderSurface>) {
		let vertices = [
			Vec2::new(0.0, 0.0),
			Vec2::new(1.0, 0.0),
			Vec2::new(1.0, 1.0),
			Vec2::new(0.0, 1.0),
		];

		let Sprite { uv_start, size, anchor_2x } = *sprite;

		let surface = surface.into().to_mat3();
		let local_size = size.to_vec2();
		let world_size = local_size * PIXEL_WORLD_SIZE * self.scale_factor;

		let texel_size = Vec2::splat(TEXEL_SIZE);

		let uv_start = uv_start.to_vec2() * texel_size;
		let uv_size = size.to_vec2() * texel_size;
		let anchor_translation = -anchor_2x.to_vec2() / local_size / 2.0;

		let color = self.tint_color.into();

		let vertices = vertices.into_iter()
			.map(|uv| {
				let v2 = (uv + anchor_translation) * world_size;
				let uv = uv * uv_size + uv_start;

				TexturedVertex {
					pos: surface * v2.extend(1.0),
					color,
					uv,
				}
			});

		self.md.extend(vertices, gfx::util::iter_fan_indices(4));
	}
}




pub struct ScreenSpriteBuilder<'md> {
	pub builder: SpriteBuilder<'md>,
	pub aspect: f32,
	pub margin: f32,
}

impl<'md> ScreenSpriteBuilder<'md> {
	pub fn build(&mut self, sprite: &Sprite, pos: Vec2) {
		let surface = gfx::OrthogonalOrientation::PositiveZ
			.to_surface_with_origin(pos.extend(0.0));

		self.builder.build_on_surface(sprite, surface);
	}

	pub fn build_with_anchor(&mut self, sprite: &Sprite, anchor: Anchor, pos: Vec2) {
		use Anchor::*;

		// See: Mat4::ortho_aspect
		let (r, t) = if self.aspect > 1.0 {
			(self.aspect, 1.0)
		} else {
			(1.0, 1.0 / self.aspect)
		};

		let ne = Vec2::new(r, t) - Vec2::splat(self.margin);
		let sw = -ne;
		let center = Vec2::zero();

		let anchor_pos = match anchor {
			Center => center,
			N => Vec2::new(center.x, ne.y),
			E => Vec2::new(ne.x, center.y),
			S => Vec2::new(center.x, sw.y),
			W => Vec2::new(sw.x, center.y),
			NE => ne,
			NW => Vec2::new(sw.x, ne.y),
			SE => Vec2::new(ne.x, sw.y),
			SW => sw,
		};

		self.build(sprite, pos + anchor_pos);
	}
}

impl<'md> std::ops::Deref for ScreenSpriteBuilder<'md> {
	type Target = SpriteBuilder<'md>;
	fn deref(&self) -> &Self::Target {
		&self.builder
	}
}

impl<'md> std::ops::DerefMut for ScreenSpriteBuilder<'md> {
	fn deref_mut (&mut self) -> &mut Self::Target {
		&mut self.builder
	}
}




pub struct SurfaceSpriteBuilder<'md> {
	pub builder: SpriteBuilder<'md>,
	pub surface: gfx::BuilderSurface,
}

impl<'md> SurfaceSpriteBuilder<'md> {
	pub fn build(&mut self, sprite: &Sprite, pos: Vec3) {
		let surface = self.surface.with_origin(pos);
		self.builder.build_on_surface(sprite, surface);
	}
}

impl<'md> std::ops::Deref for SurfaceSpriteBuilder<'md> {
	type Target = SpriteBuilder<'md>;
	fn deref(&self) -> &Self::Target {
		&self.builder
	}
}

impl<'md> std::ops::DerefMut for SurfaceSpriteBuilder<'md> {
	fn deref_mut (&mut self) -> &mut Self::Target {
		&mut self.builder
	}
}








#[derive(Debug, Copy, Clone)]
pub enum Anchor {
	Center,
	N, E, S, W,
	NE, NW, SE, SW,
}




#[derive(Debug, Copy, Clone)]
pub struct Sprite {
	pub uv_start: Vec2i,
	pub size: Vec2i,

	/// In half-pixels, so it can refer to either pixel corners or pixel centers.
	pub anchor_2x: Vec2i,
}

impl Sprite {
	pub const fn new(uv_start: Vec2i, size: Vec2i) -> Sprite {
		Sprite {
			uv_start,
			size,
			anchor_2x: size // Effectively Anchor::Center
		}
	}


	pub const fn with_pixel_anchor(self, anchor_2x: Vec2i) -> Sprite {
		Sprite {anchor_2x, .. self}
	}

	pub const fn with_anchor(self, anchor: Anchor) -> Sprite {
		use Anchor::*;

		let sw = Vec2i::zero();
		let ne = Vec2i::new(self.size.x * 2, self.size.y * 2);
		let center = self.size;

		let pixel_anchor = match anchor {
			Center => center,
			N => Vec2i::new(center.x, ne.y),
			E => Vec2i::new(ne.x, center.y),
			S => Vec2i::new(center.x, sw.y),
			W => Vec2i::new(sw.x, center.y),
			NE => ne,
			NW => Vec2i::new(sw.x, ne.y),
			SE => Vec2i::new(ne.x, sw.y),
			SW => sw,
		};

		self.with_pixel_anchor(pixel_anchor)
	}
}






#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TexturedVertex {
	pub pos: Vec3,
	pub color: Vec4,
	pub uv: Vec2,
}


impl Vertex for TexturedVertex {
	fn descriptor() -> Descriptor {
		static VERTEX_ATTRIBUTES: &'static [Attribute] = &[
			Attribute::new(0*4, AttributeType::Vec3),
			Attribute::new(3*4, AttributeType::Vec4),
			Attribute::new(7*4, AttributeType::Vec2),
		];

		Descriptor {
			attributes: VERTEX_ATTRIBUTES,
			size_bytes: std::mem::size_of::<Self>() as u32,
		}
	}
}
