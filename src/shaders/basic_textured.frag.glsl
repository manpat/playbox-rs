#version 450

in vec2 v_uv;

layout(binding=0) uniform sampler2D u_texture;

layout(location=0) out vec4 out_color;

void main() {
	out_color = texture(u_texture, v_uv);
}