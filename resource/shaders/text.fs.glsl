
in Vertex {
	vec4 v_color;
	vec2 v_uv;
};

layout(location=0, index=0) out vec4 o_color;
layout(location=0, index=1) out vec4 o_coverage;

layout(binding=0) uniform sampler2D u_texture;

void main() {
	// NOTE: not subpixel for now, but set up for it
	vec3 coverage = vec3(texture(u_texture, v_uv).r);

	o_color = v_color * vec4(coverage * v_color.a, 1);
	o_coverage = vec4(coverage * v_color.a, v_color.a);
}


// https://acko.net/blog/subpixel-distance-transform/
// https://github.com/arkanis/gl-4.5-subpixel-text-rendering/blob/17f4af4df858c52092ccad7c4292e7e4cd08091b/main.c
// http://arkanis.de/weblog/2023-08-14-simple-good-quality-subpixel-text-rendering-in-opengl-with-stb-truetype-and-dual-source-blending