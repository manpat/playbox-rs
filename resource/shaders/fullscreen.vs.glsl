

out OutVertex {
	vec4 v_color;
	vec2 v_uv;
};


const vec2[] c_vertices = {
	vec2(-1.0,-1.0),
	vec2( 1.0,-1.0),
	vec2( 1.0, 1.0),

	vec2(-1.0,-1.0),
	vec2( 1.0, 1.0),
	vec2(-1.0, 1.0),
};


void main() {
	vec2 vertex = c_vertices[gl_VertexID % 6];

	gl_Position = vec4(vertex, 0.0, 1.0);

	v_color = vec4(1.0);
	v_uv = vertex * 0.5 + 0.5;
}