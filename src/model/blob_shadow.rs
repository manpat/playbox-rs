use toybox::prelude::*;

pub struct BlobShadowModel {
	pub shadow_casters: Vec<BlobShadowCaster>,
}

pub struct BlobShadowCaster {
	pub position: Vec3,
	pub scale: f32,
}


impl BlobShadowModel {
	pub fn new() -> BlobShadowModel {
		BlobShadowModel {
			shadow_casters: Vec::new(),
		}
	}

	pub fn add(&mut self, position: Vec3, scale: f32) {
		self.shadow_casters.push(BlobShadowCaster {
			position,
			scale,
		});
	}

	pub fn clear(&mut self) {
		self.shadow_casters.clear();
	}
}