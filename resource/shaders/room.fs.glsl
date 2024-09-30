
in Vertex {
	vec4 v_color;
	vec2 v_uv;
	flat uint v_texture_index;
};

out vec4 o_color;

layout(binding=0) uniform sampler2DArray u_texture;


// TODO(pat.m): use an atlas instead of a fixed size texture array
// struct TextureData {
// 	uvec2 offset;
// 	uvec2 size;
// 	uint layer;
// };

// layout(binding=5) readonly buffer TD {
// 	TextureData s_textures[];
// };


void main() {
	ivec3 image_dimensions = textureSize(u_texture, 0);
	ivec2 image_size = image_dimensions.xy;
	uint num_textures = uint(image_dimensions.z);

	if (v_texture_index > 0) {
		ivec2 texel_coord = ivec2(v_uv) % image_size;

		// Cheeky texture variation. should be configurable 
		ivec2 chunk_coord = ivec2(v_uv) / image_size;
		uint texture_index = v_texture_index + (chunk_coord.x + chunk_coord.y) % 2;
		uint real_texture_index = min(texture_index, num_textures)-1;

		o_color = texelFetch(u_texture, ivec3(texel_coord, real_texture_index), 0) * v_color;
	} else {
		o_color = v_color;
	}
}