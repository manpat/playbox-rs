
in OutVertex {
	vec4 v_color;
	vec2 v_uv;
	vec3 v_local_pos;
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

struct Light {
	vec3 local_pos;
	float radius;
	vec3 color;
	float power;

	vec4 plane_0;
	vec4 plane_1;
};

layout(binding=2) readonly buffer RoomInfo {
	uint u_first_light;
	uint u_num_lights;
};

layout(binding=3) readonly buffer LightInfo {
	Light s_lights[];
};

// https://gist.github.com/patriciogonzalezvivo/670c22f3966e662d2f83
float hash(float n) { return fract(sin(n) * 1e4); }
float hash(vec2 p) { return fract(1e4 * sin(17.0 * p.x + p.y * 0.1) * (0.1 + abs(sin(p.y * 13.0 + p.x)))); }



void main() {
	ivec3 image_dimensions = textureSize(u_texture, 0);
	ivec2 image_size = image_dimensions.xy;
	uint num_textures = uint(image_dimensions.z);

	vec3 diffuse = vec3(0.0);

	if (v_texture_index > 0) {
		ivec2 texel_coord = ivec2(v_uv) % image_size;

		// Cheeky texture variation. should be configurable 
		ivec2 chunk_coord = ivec2(v_uv) / image_size;
		uint texture_index = v_texture_index + uint(hash(vec2(chunk_coord) * 3.123) * 2.0);
		uint real_texture_index = min(texture_index, num_textures)-1;

		diffuse = texelFetch(u_texture, ivec3(texel_coord, real_texture_index), 0).rgb * v_color.rgb;
	} else {
		diffuse = v_color.rgb;
	}

	vec3 lighting = vec3(0.0);

	// Collect lighting
	for (uint light_idx = u_first_light; light_idx < u_first_light + u_num_lights; light_idx++) {
		Light light = s_lights[light_idx];

		float d0 = dot(light.plane_0, vec4(v_local_pos, -1.0));
		float d1 = dot(light.plane_1, vec4(v_local_pos, -1.0));

		const float feathering = 0.01;

		float occlusion = min(smoothstep(-feathering, feathering, d0), smoothstep(-feathering, feathering, d1));

		float value = max(1.0 - length(light.local_pos - v_local_pos) / light.radius, 0.0) * occlusion;

		value *= light.power;

		value = ceil(value*4.0)/4.0;
		value *= value;

		lighting += diffuse * light.color * value;
	}

	o_color.rgb = diffuse.rgb + lighting;
	o_color.a = 1.0;
}