
const vec2 points[3] = vec2[](
	vec2(-0.5, -0.5),
	vec2( 0.0,  0.5),
	vec2( 0.5, -0.5)
);

out float v_instance;


layout(binding=0) uniform U {
	float u_time_sin;
};

void main() {
	float offset = gl_InstanceID/10.0;
	gl_Position = vec4(points[gl_VertexID % 3] + vec2(sin(u_time_sin - offset*2.0), offset-0.2), offset, 1.0);
	gl_PointSize = 10.0;

	v_instance = float(gl_InstanceID) / 10.0;
}