#version 450
#import global

layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_color;

layout(std430, row_major, binding=0) buffer InstanceData {
	mat4x3 u_transforms[];
};


out vec3 v_color;

void main() {
	vec3 worldspace_pos = u_transforms[gl_InstanceID] * vec4(a_pos, 1.0);
	gl_Position = u_projection_view * vec4(worldspace_pos, 1.0);
	v_color = a_color;
}