#version 450

in vec2 v_uv;

layout(binding=0) uniform sampler2D u_scene;
layout(binding=1) uniform sampler2D u_foreground;

layout(binding=2) uniform sampler2D u_scene_depth;
layout(binding=3) uniform sampler2D u_foreground_depth;

layout(location=0) out vec4 out_color;

void main() {
	vec4 scene_color = texture(u_scene, v_uv);
	vec4 foreground_color = texture(u_foreground, v_uv);

	float scene_depth = texture(u_scene_depth, v_uv).r;
	float foreground_depth = texture(u_foreground_depth, v_uv).r;

	const float depth_pass = float(scene_depth > foreground_depth);
	const float alpha = depth_pass * foreground_color.a;

	out_color = mix(scene_color, foreground_color, alpha);
	gl_FragDepth = mix(scene_depth, foreground_depth, alpha);
}