

out float v_instance;


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

	vec3 world_pos = vec3(u_points[gl_VertexID % u_points.length()] + vec2(sin(u_time - offset*2.0), offset-0.2), offset);
	gl_Position = u_projection_view * vec4(world_pos, 1.0);
	gl_PointSize = 10.0;

	float clip_dist = sin(u_time*3.0 + offset) * 0.3 + 0.6;
	float clip_dist2 = cos(u_time*3.0 + offset) * 0.3 + 0.6;

	gl_ClipDistance[0] = world_pos.x + clip_dist;
	gl_ClipDistance[1] = -world_pos.x + clip_dist;
	gl_ClipDistance[2] = world_pos.y + clip_dist2;
	gl_ClipDistance[3] = -world_pos.y + clip_dist2;

	v_instance = float(gl_InstanceID) / 10.0;
}