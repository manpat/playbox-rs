#version 450


layout(std140, row_major, binding = 1) uniform PostEffectData {
	mat4 u_projection;
	mat4 u_projection_inverse;
	vec4 u_pulse_color;
	vec4 u_fog_color;
	float u_fog_power;
	float u_fog_dist;
	float u_fog_steps;
};




in vec2 v_uv;

layout(binding=0) uniform sampler2D u_scene;
layout(binding=1) uniform sampler2D u_scene_depth;

layout(location=0) out vec4 out_color;



void main() {
	vec4 scene_color = texture(u_scene, v_uv);
	float scene_depth = texture(u_scene_depth, v_uv).r;

	vec3 ndc_pos = vec3(v_uv, scene_depth) * 2.0 - 1.0;
	vec4 view_pos = u_projection_inverse * vec4(ndc_pos, 1.0);
	view_pos /= view_pos.w;

	vec3 final_color = scene_color.rgb;

	view_pos.rgb = floor(view_pos.rgb*u_fog_steps)/u_fog_steps;

	float fog_amt = length(view_pos.xz)/u_fog_dist;

	// fog_amt = floor(fog_amt*u_fog_steps)/u_fog_steps;

	fog_amt = clamp(fog_amt, 0.0, 1.0);
	fog_amt = pow(fog_amt, u_fog_power);
	fog_amt = clamp(fog_amt, 0.0, 0.95);
	final_color = mix(final_color * (vec3(1.0) - u_fog_color.rgb), final_color * u_fog_color.rgb, fog_amt);
	final_color = mix(final_color, u_pulse_color.rgb, u_pulse_color.a);

	out_color = vec4(final_color, 1.0);
}