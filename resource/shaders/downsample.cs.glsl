layout(local_size_x=8, local_size_y=8) in;

layout(binding=0) uniform sampler2D u_source_mip;
layout(binding=1, rgba16f) writeonly uniform image2D u_target_mip;

void main() {
	ivec2 texel_coord = ivec2(gl_GlobalInvocationID.xy);

	ivec2 target_size = imageSize(u_target_mip);
	if (any(greaterThanEqual(texel_coord, target_size))) {
		return;
	}

	vec2 texel_size = 1.0 / vec2(target_size);
	vec2 uv = (vec2(texel_coord) + 0.5) * texel_size;

	vec3 color = vec3(0.0);
	vec3 sign = vec3(1, -1, 0);

	// Center
	color += texture(u_source_mip, uv).rgb * 0.25;

	// Adjacent
	color += texture(u_source_mip, uv + texel_size * sign.xz).rgb * 0.125;
	color += texture(u_source_mip, uv + texel_size * sign.yz).rgb * 0.125;
	color += texture(u_source_mip, uv + texel_size * sign.zx).rgb * 0.125;
	color += texture(u_source_mip, uv + texel_size * sign.zy).rgb * 0.125;

	// Corner
	color += texture(u_source_mip, uv + texel_size * sign.xx).rgb * 0.0625;
	color += texture(u_source_mip, uv + texel_size * sign.xy).rgb * 0.0625;
	color += texture(u_source_mip, uv + texel_size * sign.yx).rgb * 0.0625;
	color += texture(u_source_mip, uv + texel_size * sign.yy).rgb * 0.0625;

	imageStore(u_target_mip, texel_coord, vec4(color, 1.0));
}