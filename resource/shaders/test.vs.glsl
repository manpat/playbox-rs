

out Vertex {
	vec4 v_color;
	vec2 v_uv;
};

layout(binding=0) uniform U {
	float u_time;
};

layout(binding=1) uniform P {
	mat4 u_projection_view;
};

layout(binding=2) uniform V {
	vec2 u_points[3];
};

void main() {
	float offset = gl_InstanceID/10.0;

	vec2 base_pos = u_points[gl_VertexID % u_points.length()];
	v_uv = base_pos * 3.0 + 0.5 + vec2(u_time/2.0, sin(u_time + 3.0 * offset));

	vec3 world_pos = vec3(base_pos + vec2(sin(u_time - offset*2.0), offset-0.2), offset);
	gl_Position = u_projection_view * vec4(world_pos, 1.0);
	gl_PointSize = 10.0;

	float clip_dist = sin(u_time*1.2 + offset) * 0.3 + 0.8;
	float clip_dist2 = cos(u_time*1.2 + offset) * 0.3 + 0.8;

	gl_ClipDistance[0] = world_pos.x + clip_dist;
	gl_ClipDistance[1] = -world_pos.x + clip_dist;
	gl_ClipDistance[2] = world_pos.y + clip_dist2;
	gl_ClipDistance[3] = -world_pos.y + clip_dist2;

	v_color = vec4(1.0);
}