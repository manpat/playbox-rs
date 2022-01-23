#version 450
#import gbuffer_particle

layout(local_size_x=8, local_size_y=8, local_size_z=1) in;




void update_particle(uint particle_index) {
	Particle particle = g_particles[particle_index];

	particle.position += particle.velocity * 0.05;
	// particle.velocity += -particle.velocity.zxy * 0.03 - (particle.position + vec3(0.0, 2.0, 0.0)) * 0.002;
	// particle.velocity.y += 0.01;
	particle.velocity *= 0.995;
	// particle.color.rgb = mix(particle.color.rgb, vec3(0.5), vec3(0.0001));

	g_particles[particle_index] = particle;
}


void main() {
	const uvec3 gid = gl_WorkGroupID;
	const uint global_idx = gid.x + gid.y * gl_NumWorkGroups.x + gid.z * gl_NumWorkGroups.x * gl_NumWorkGroups.y;

	const uint workgroup_size = gl_WorkGroupSize.x * gl_WorkGroupSize.y * gl_WorkGroupSize.z;

	const uint idx = gl_LocalInvocationIndex + workgroup_size * global_idx;

	if (idx < g_particles.length()) {
		update_particle(idx);
	}
}