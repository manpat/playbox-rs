#version 450

in vec4 v_color;

layout(location=0) out vec4 out_color;


float angled_stripes(in float angle_degrees, in float size) {
	const float angle = radians(angle_degrees);
	vec2 direction = vec2(cos(angle), sin(angle));
	float dist = dot(gl_FragCoord.xy, direction) / size;
	return mod(dist, 1.0);
}


void main() {
	const float dist_a = angled_stripes(-60, 5.0);
	const float dist_b = angled_stripes(30 - v_color.a * 10.0, 1.0);

	const float dist = max(dist_a, dist_b);

	if (dist > v_color.a) {
		discard;
	}

	out_color = vec4(v_color.rgb, 1.0);
}