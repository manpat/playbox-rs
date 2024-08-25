layout(local_size_x=8, local_size_y=8) in;


layout(binding=0) uniform P {
	mat4 u_projection_view;
	mat4 u_inverse_projection;
};

layout(binding=1) uniform F {
	vec4 u_fog_color;
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

	// view.xyz = round(view.xyz*4.0)/4.0;

	float distance = length(view.xyz);

	float fog_factor = 1.0 - pow(clamp(1.0 - distance / 10.0, 0.0, 1.0), 10.0);

	vec3 fog_color = u_fog_color.rgb;
	// vec3 fog_absorption = vec3(1.0) - fog_color;


	texel.rgb = mix(texel.rgb, fog_color, fog_factor);

	// vec3 softlight = (1.0 - 2.0 * texel.rgb) * fog_color * fog_color + 2.0 * fog_color * texel.rgb;
	// texel.rgb = overlay;
	// texel.rgb = mix(texel.rgb, softlight, fog_factor);


	texel.a = 1.0;

	// texel.rgb -= texel.rgb * fog_absorption * pow(distance / 4.0, 0.7);
	// texel.rgb -= texel.rgb * fog_absorption * max(distance + 50.0, 0.0) / 61.0;

	texel.rgb += fog_color * pow(distance / 10.0, 2.0);

	// texel.rgb = mix(texel.rgb, vec3(1.0) - (vec3(1.0) - texel.rgb * u_fog_color.rgb * fog_factor), fog_factor);


	// texel.rgb += fog_color * fog_factor;

	imageStore(u_image, texel_uv, texel);
}