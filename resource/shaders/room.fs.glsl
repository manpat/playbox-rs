
in Vertex {
	vec4 v_color;
	vec2 v_uv;
	flat uint v_texture_index;
};

out vec4 o_color;

layout(binding=0) uniform sampler2DArray u_texture;

void main() {
	ivec2 image_size = textureSize(u_texture, 0).xy;
	// vec2 texel_size = 1.0 / vec2(image_size);
	// o_color = texture(u_texture, vec3(texel_size*v_uv, float(v_texture_index))) * v_color;

	if (v_texture_index > 0) {
		ivec2 texel_coord = ivec2(v_uv) % image_size;
		o_color = texelFetch(u_texture, ivec3(texel_coord, v_texture_index-1), 0) * v_color;
	} else {
		o_color = v_color;
	}
}