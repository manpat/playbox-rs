layout(local_size_x=8, local_size_y=8) in;

layout(binding=0) uniform sampler2D u_source_mip;
layout(binding=1, rgba16f) writeonly uniform image2D u_target_mip;

void main() {
	ivec2 texel_coord = ivec2(gl_GlobalInvocationID.xy);

	ivec2 target_size = imageSize(u_target_mip);
	if (any(greaterThanEqual(texel_coord, target_size))) {
		return;
	}

	// ivec2 source_size = textureSize(u_source_mip, 0);

	vec4 color = texture(u_source_mip, (vec2(texel_coord) + 0.5) / vec2(target_size));

	imageStore(u_target_mip, texel_coord, color);
}