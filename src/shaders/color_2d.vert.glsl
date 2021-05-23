#version 450
#import global

layout(location=0) in vec2 a_pos;
layout(location=1) in vec3 a_color;

out vec3 v_color;

void main() {
	gl_PointSize = 3.0;
	gl_Position = u_ui_projection_view * vec4(a_pos, 0.0, 1.0);
	v_color = a_color;
}