layout(local_size_x=8, local_size_y=8) in;


layout(binding=0) uniform P {
	mat4 u_projection_view;
	mat4 u_inverse_projection;
};

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

	vec4 ndc = vec4(vec2(texel_uv) / vec2(image_size) * 2.0 - 1.0, depth, 1.0);

	vec4 view = u_inverse_projection * ndc;
	view.xyz /= view.w;

	float distance = length(view.xyz);

	texel *= pow(clamp(1.0 - distance / 20.0, 0.0, 1.0), 8.0);

	imageStore(u_image, texel_uv, texel);
}