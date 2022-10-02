#version 450
#import global

layout(location=0) in vec3 a_pos;
layout(location=1) in uint a_pattern_index;
layout(location=2) in vec4 a_color_0;
layout(location=3) in vec4 a_color_1;
layout(location=4) in uint a_shape_id;

flat out uint v_pattern_index;
flat out uint v_shape_id;
out vec4 v_color_0;
out vec4 v_color_1;

void main() {
	gl_Position = u_projection_view * vec4(a_pos, 1.0);
	v_pattern_index = a_pattern_index;
	v_shape_id = a_shape_id;
	v_color_0 = a_color_0;
	v_color_1 = a_color_1;
}