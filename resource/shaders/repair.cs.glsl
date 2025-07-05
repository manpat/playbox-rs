layout(local_size_x=8, local_size_y=8) in;

layout(binding=0, rgba16f) uniform image2D u_image;
layout(binding=1) uniform sampler2D u_depth;

void main() {
	ivec2 texel_uv = ivec2(gl_GlobalInvocationID.xy);

	ivec2 image_size = imageSize(u_image);
	if (any(greaterThanEqual(texel_uv, image_size))) {
		return;
	}

	float depth = texelFetch(u_depth, texel_uv, 0).r;
	if (depth == 1.0) {
		// We're assuming that the next texel over isn't empty.
		// If it is its technically UB but w/e.
		vec4 replacement = imageLoad(u_image, texel_uv + ivec2(1, 0));
		imageStore(u_image, texel_uv, replacement);
	}
}