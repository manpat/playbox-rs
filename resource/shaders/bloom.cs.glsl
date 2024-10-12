layout(local_size_x=8, local_size_y=8) in;

layout(binding=0) uniform sampler2D u_blurred_image;
layout(binding=1, rgba16f) uniform image2D u_composite_target;

void main() {
	ivec2 texel_coord = ivec2(gl_GlobalInvocationID.xy);

	ivec2 target_size = imageSize(u_composite_target);
	if (any(greaterThanEqual(texel_coord, target_size))) {
		return;
	}

	vec2 uv = (vec2(texel_coord) + 0.5) / vec2(target_size);

	vec3 base_color = imageLoad(u_composite_target, texel_coord).rgb;
	vec3 blurred_color = texture(u_blurred_image, uv).rgb;

	// vec3 final_color = mix(base_color, blurred_color, 0.5);
	vec3 final_color = base_color + blurred_color;

	imageStore(u_composite_target, texel_coord, vec4(final_color, 1.0));
}