

out OutVertex {
	vec2 v_uv;
};

layout(binding=1) uniform P {
	mat4 u_projection_view;
};


struct Vertex {
	vec4 pos;
	vec2 uv;
};

layout(binding=0) readonly buffer V {
	Vertex s_vertices[];
};

void main() {
	Vertex vertex = s_vertices[gl_VertexID];

	gl_Position = u_projection_view * vec4(vertex.pos.xyz, 1.0);
	gl_PointSize = 10.0;

	v_uv = vertex.uv;
}