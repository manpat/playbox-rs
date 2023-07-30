
const vec2 points[3] = vec2[](
	vec2(-0.5, -0.5),
	vec2( 0.0,  0.5),
	vec2( 0.5, -0.5)
);

out float v_instance;

void main() {
	gl_Position = vec4(points[gl_VertexID % 3] + vec2(0.0, float(gl_InstanceID) / 10.0), 0.0, 1.0);
	gl_PointSize = 10.0;

	v_instance = float(gl_InstanceID) / 10.0;
}