layout(local_size_x=8, local_size_y=8) in;


layout(binding=0, rgba16f) uniform image2D u_image;
layout(binding=1) uniform sampler2D u_depth;

void main() {
	ivec2 texel_uv = ivec2(gl_GlobalInvocationID.xy);

	ivec2 image_size = imageSize(u_image);
	if (any(greaterThanEqual(texel_uv, image_size))) {
		return;
	}

	vec4 texel = imageLoad(u_image, texel_uv);
	float depth = texelFetch(u_depth, texel_uv, 0).r;

	float ndc_depth = depth * 2.0 - 1.0;
	float linear_depth = (2.0 * 0.01 * 100.0) / (100.0 + 0.01 - ndc_depth * (100.0 - 0.01));

	texel *= pow(clamp(1.0 - linear_depth/20.0, 0.0, 1.0), 2.0);

	imageStore(u_image, texel_uv, texel);
}