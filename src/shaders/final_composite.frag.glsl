#version 450

in vec2 v_uv;

layout(binding=0) uniform sampler2D u_texture0;
layout(binding=1) uniform sampler2D u_texture1;

layout(location=0) out vec4 out_color;

void main() {
	vec4 color_0 = texture(u_texture0, v_uv);
	vec4 color_1 = texture(u_texture1, v_uv);
	out_color = vec4(max(color_0.rgb, color_1.rgb), 1.0);
}