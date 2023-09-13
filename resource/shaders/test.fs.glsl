
in Vertex {
	vec2 v_uv;
	float v_instance;
};

out vec4 o_color;

layout(binding=0) uniform sampler2D u_texture; 

void main() {
	o_color = texture(u_texture, v_uv);
}