#version 450

in vec4 v_color;
// in uint v_particle_id;

layout(location=0) out vec4 out_color;

void main() {
	if (length(gl_PointCoord - vec2(0.5)) > 0.5) {
		discard;
	}

	out_color = v_color;
}