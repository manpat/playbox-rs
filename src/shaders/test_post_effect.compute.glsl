#version 450

layout(local_size_x=8, local_size_y=8, local_size_z=1) in;

layout(binding = 0, r11f_g11f_b10f) uniform image2D u_image;
// layout(binding = 0, rgba8) uniform image2D u_image;

void main() {
	const ivec2 uv = ivec2(gl_GlobalInvocationID.xy);
	const ivec2 image_size = imageSize(u_image);

	if (any(greaterThan(uv, image_size))) {
		return;
	}

	const float centre_dist = dot(abs(uv - image_size/2), ivec2(1));

	vec4 value = imageLoad(u_image, uv);

	if (centre_dist > 400.0) {
		value.rgb = vec3(1.0) - value.rgb;
	}

	imageStore(u_image, uv, value);
}