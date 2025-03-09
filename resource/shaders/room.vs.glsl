
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

struct Instance {
	mat4x4 transform;
	vec4 plane_0;
	vec4 plane_1;
	vec4 plane_2;
};

layout(binding=0) readonly buffer V {
	Vertex s_vertices[];
};

layout(binding=1, std430) readonly buffer M {
	Instance s_instances[];
};


out OutVertex {
	layout(location=0) vec4 v_color;
	layout(location=1) vec2 v_uv;
	layout(location=2) vec3 v_local_pos;
	layout(location=3) flat uint v_texture_index;
};

void main() {
	Vertex vertex = s_vertices[gl_VertexID];
	Instance instance = s_instances[gl_InstanceID];

	vec3 world_pos = (instance.transform * vec4(vertex.pos, 1.0)).xyz;
	gl_Position = u_projection_view * vec4(world_pos, 1.0);

	gl_ClipDistance[0] = dot(instance.plane_0.xyz, vertex.pos) - instance.plane_0.w;
	gl_ClipDistance[1] = dot(instance.plane_1.xyz, vertex.pos) - instance.plane_1.w;
	gl_ClipDistance[2] = dot(instance.plane_2.xyz, vertex.pos) - instance.plane_2.w;
	gl_ClipDistance[3] = 1.0;

	v_local_pos = vertex.pos;

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