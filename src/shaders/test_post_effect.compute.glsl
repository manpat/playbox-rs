#version 450

layout(local_size_x=16, local_size_y=16, local_size_z=1) in;

layout(binding = 0, r11f_g11f_b10f) uniform image2D u_image;
// layout(binding = 0, rgba8) uniform image2D u_image;

shared uint g_counter;
shared vec4 g_samples[16*16];

uint uv_to_samples_index(in ivec2 uv) {
	return uv.x + uv.y * 16;
}

void main() {
	const ivec2 uv = ivec2(gl_GlobalInvocationID.xy);
	const ivec2 local_uv = ivec2(gl_LocalInvocationID.xy);

	if (gl_LocalInvocationIndex == 0) {
		g_counter = 0;
	}

	g_samples[uv_to_samples_index(local_uv)] = imageLoad(u_image, uv);

	barrier();
	memoryBarrierShared();

	const ivec2 image_size = imageSize(u_image);
	if (any(greaterThan(uv, image_size))) {
		return;
	}

	// const float centre_dist = dot(abs(uv - image_size/2), ivec2(1));
	const float centre_dist = length(uv - image_size/2);

	vec4 value = vec4(0.0, 0.0, 0.0, 1.0);

	if (centre_dist > 500.0) {
		uint my_counter = (gl_WorkGroupID.y ^ gl_WorkGroupID.x + atomicAdd(g_counter, 1)); // atomicAdd(g_counter, 1);

		ivec2 uv_flipped_x = local_uv;
		ivec2 uv_flipped_y = local_uv;
		ivec2 uv_modulo_x = local_uv;
		ivec2 uv_modulo_y = local_uv;
		uv_flipped_x.x = int(gl_WorkGroupSize.x) - uv_flipped_x.x - 1;
		uv_flipped_y.y = int(gl_WorkGroupSize.y) - uv_flipped_y.y - 1;
		uv_modulo_x.x = (uv_modulo_x.x + int(my_counter)) % int(gl_WorkGroupSize.x);
		uv_modulo_y.y = (uv_modulo_y.y + int(my_counter)) % int(gl_WorkGroupSize.y);

		const vec3 value_a = g_samples[uv_to_samples_index(local_uv)].rgb;
		const vec3 value_b = g_samples[uv_to_samples_index(uv_flipped_x)].rgb;
		const vec3 value_c = g_samples[uv_to_samples_index(uv_flipped_y)].rgb;
		const vec3 value_d = g_samples[uv_to_samples_index(uv_modulo_x)].rgb;
		const vec3 value_e = g_samples[uv_to_samples_index(uv_modulo_y)].rgb;
		value.rgb = (value_a + /* value_b.rgr + value_c.brb */ + pow(value_d + value_e, vec3(0.9))) / 4.0;
		value.rgb = pow(vec3(1.0) - value.rgb, vec3(2.0));

		// if (my_counter % 8 < 2) {
		// 	value.rgb = vec3(1.0) - value.gbr;
		// } else if (my_counter / 8 < 2) {
		// 	value.rgb = vec3(1.0) - value.brg;
		// } else if (my_counter > 200) {
		// 	value.rgb = value.bgr;
		// }
	} else {
		value = g_samples[uv_to_samples_index(local_uv)];
	}

	imageStore(u_image, uv, value);
}