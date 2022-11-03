#version 450

in vec4 v_color;
in vec2 v_uv;

layout(binding=0) uniform sampler2D u_texture;

layout(location=0) out vec4 out_color;

void main() {
	vec4 tex_color = texture(u_texture, v_uv);
	out_color = tex_color * v_color;

	if (out_color.a < 0.5) {
		discard;
	}
}