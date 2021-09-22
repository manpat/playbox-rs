#version 450

out vec2 v_uv;

const vec2[4] g_uvs = {
	{0.0, 0.0},
	{1.0, 0.0},
	{1.0, 1.0},
	{0.0, 1.0},
};

const vec2[4] g_positions = {
	{-1.0, -1.0},
	{1.0, -1.0},
	{1.0, 1.0},
	{-1.0, 1.0},
};

const uint g_indices[6] = {0, 1, 2, 0, 2, 3};

void main() {
	const uint index = g_indices[gl_VertexID];

	gl_Position = vec4(g_positions[index], 0.0, 1.0);
	v_uv = g_uvs[index];
}