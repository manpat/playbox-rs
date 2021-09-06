use toybox::prelude::*;


#[derive(Copy, Clone, Debug)]
pub struct Ray {
	pub position: Vec3,
	pub direction: Vec3,
}

fn transform_ray(ray: &Ray, transform: &Mat3x4) -> Ray {
	let position = *transform * ray.position.extend(1.0);
	let direction = *transform * ray.direction.extend(0.0);

	Ray {
		position: position.to_vec3(),
		direction: direction.to_vec3(),
	}
}



pub fn scene_raycast(scene: &toy::SceneRef<'_>, ray: &Ray) -> Option<Vec3> {
	use common::ordified::*;

	let castable_entities = scene.entities()
		.filter(|e| !e.name.contains('_'));

	castable_entities
		.filter_map(|e| entity_raycast(&e, ray))
		.min_by_key(|rc| rc.dot(ray.direction).ordify())
}

fn entity_raycast(entity: &toy::EntityRef<'_>, ray: &Ray) -> Option<Vec3> {
	use common::ordified::*;

	let transform = entity.transform();
	let inv_transform = transform.inverse();
	let ray_local = transform_ray(ray, &inv_transform);

	let mesh_data = entity.mesh_data()?;

	let triangles = mesh_data.indices.array_chunks::<3>()
		.map(|&[i0, i1, i2]| [
			mesh_data.positions[i0 as usize],
			mesh_data.positions[i1 as usize],
			mesh_data.positions[i2 as usize],
		]);

	let intersections = triangles
		.filter_map(|triangle| ray_triangle_intersect(&ray_local, &triangle, false));

	let min_hit = intersections.min_by_key(|intersection| intersection.t.ordify())?;
	if min_hit.t >= 0.0 {
		let local_hit = ray_local.position + ray_local.direction * min_hit.t;
		Some(transform * local_hit)
	} else {
		None
	}
}


struct IntersectResult {
	t: f32,
	// u: f32,
	// v: f32,
}

fn ray_triangle_intersect(ray: &Ray, triangle: &[Vec3; 3], cull_backface: bool) -> Option<IntersectResult> {
	let v0v1 = triangle[1] - triangle[0];
	let v0v2 = triangle[2] - triangle[0];
	let pvec = ray.direction.cross(v0v2);
	let det = v0v1.dot(pvec);

	if cull_backface {
		// if the determinant is negative the triangle is backfacing
		// if the determinant is close to 0, the ray misses the triangle
		if det < 0.0001 {
			return None
		}
	} else {
		// ray and triangle are parallel if det is close to 0
		if det.abs() < 0.0001 {
			return None
		}
	}

	let inv_det = 1.0 / det;
 
	let tvec = ray.position - triangle[0];
	let u = tvec.dot(pvec) * inv_det;
	if u < 0.0 || u > 1.0 {
		return None
	}
 
	let qvec = tvec.cross(v0v1);
	let v = ray.direction.dot(qvec) * inv_det;
	if v < 0.0 || u + v > 1.0 {
		return None
	}
 
	let t = v0v2.dot(qvec) * inv_det;
 
	Some(IntersectResult {t, /*u, v*/})
} 