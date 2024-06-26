layout(local_size_x=8, local_size_y=8) in;


layout(binding=0, rgba16f) uniform image2D u_image;

void main() {
	ivec2 texel_uv = ivec2(gl_GlobalInvocationID.xy);

	ivec2 image_size = imageSize(u_image);
	if (any(greaterThanEqual(texel_uv, image_size))) {
		return;
	}

	vec4 texel = imageLoad(u_image, texel_uv);

	if ((texel_uv.x + texel_uv.y)/16 % 2 > 0) {
		texel.rgb *= 0.9;
	}

	imageStore(u_image, texel_uv, texel);
}