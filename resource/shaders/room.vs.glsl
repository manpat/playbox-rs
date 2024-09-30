
struct Vertex {
	vec3 pos;
	int uv_packed;
	uvec2 color_packed;
	uint texture_index;
	uint _padding;
};


layout(binding=0) uniform P {
	mat4 u_projection_view;
};

layout(binding=1) uniform M {
	mat4x3 u_model;
	vec4 u_plane_0;
	vec4 u_plane_1;
	vec4 u_plane_2;
};

layout(binding=0) readonly buffer V {
	Vertex s_vertices[];
};


out OutVertex {
	vec4 v_color;
	vec2 v_uv;
	flat uint v_texture_index;
};

void main() {
	Vertex vertex = s_vertices[gl_VertexID];

	vec3 world_pos = u_model * vec4(vertex.pos, 1.0);
	gl_Position = u_projection_view * vec4(world_pos, 1.0);

	gl_ClipDistance[0] = dot(u_plane_0.xyz, vertex.pos) - u_plane_0.w;
	gl_ClipDistance[1] = dot(u_plane_1.xyz, vertex.pos) - u_plane_1.w;
	gl_ClipDistance[2] = dot(u_plane_2.xyz, vertex.pos) - u_plane_2.w;
	gl_ClipDistance[3] = 1.0;

	v_color = vec4(
		unpackUnorm2x16(vertex.color_packed.x),
		unpackUnorm2x16(vertex.color_packed.y)
	);

	v_uv = vec2(
		float(bitfieldExtract(vertex.uv_packed, 0, 16))/8.0,
		float(bitfieldExtract(vertex.uv_packed, 16, 16))/8.0
	);

	v_texture_index = vertex.texture_index;
}