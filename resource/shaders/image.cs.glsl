layout(local_size_x=1) in;


layout(binding=0, rgba8) uniform image2D u_image;

void main() {
	ivec2 texel_uv = ivec2(gl_GlobalInvocationID.xy);
	vec4 texel = imageLoad(u_image, texel_uv);

	float alpha = 0.1;
	float c = cos(alpha);
	float s = sin(alpha);
	float z = 0.0;
	float o = 1.0;

	mat3 txform = mat3(
		c, z, -s,
		z, o, z,
		s, z, c
	);

	texel.rgb = texel.rgb * 2.0 - 1.0;
	texel.rgb = txform * texel.rgb;
	texel.rgb = texel.rgb * 0.5 + 0.5;

	imageStore(u_image, texel_uv, texel);
}