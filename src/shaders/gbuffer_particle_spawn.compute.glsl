#version 450
#import global
#import gbuffer_particle

layout(local_size_x=8, local_size_y=8, local_size_z=1) in;



layout(binding = 0) uniform sampler2D u_color;
layout(binding = 1) uniform sampler2D u_depth;


uint hash( uint x ) {
	x += ( x << 10u );
	x ^= ( x >>  6u );
	x += ( x <<  3u );
	x ^= ( x >> 11u );
	x += ( x << 15u );
	return x;
}


void spawn_particle(uint particle_index) {
	uint sample_idx = hash(particle_index + hash(g_workgroup_allocations));
	uint sample_idx2 = hash(particle_index * 5 ^ sample_idx);

	vec2 sample_pos;
	sample_pos.x = mod(float(sample_idx % 1553) / 577.0, 1.0);
	sample_pos.y = mod(float(sample_idx2 / 1553) / 577.0, 1.0);

	float depth = texture(u_depth, sample_pos).r;

	vec4 world_pos = vec4(1.0/0.0);
	vec4 color = vec4(0.0);

	if (depth < 1.0) {
		vec3 ndc_pos = vec3(sample_pos, depth) * 2.0 - 1.0;
		world_pos = u_projection_view_inverse * vec4(ndc_pos, 1.0);
		world_pos.xyz /= world_pos.w;

		color = texture(u_color, sample_pos);

		vec3 velocity;
		velocity.x = float(hash(particle_index * 227) % 100) - 50.0;
		velocity.y = float(hash(particle_index * 727) % 100) - 50.0;
		velocity.z = float(hash(particle_index * 467) % 100) - 50.0;

		Particle particle;
		particle.position = world_pos.xyz;
		particle.velocity = normalize(velocity) * 0.5;
		particle.color = color;
		g_particles[particle_index] = particle;
	}
}



shared uint group_head_ptr;


void main() {
	const uint workgroup_size = gl_WorkGroupSize.x * gl_WorkGroupSize.y * gl_WorkGroupSize.z;

	if (gl_LocalInvocationIndex == 0) {
		const uint allocated_head_ptr = atomicAdd(g_head_ptr, workgroup_size);
		const uint current_head_ptr = allocated_head_ptr + workgroup_size;
		atomicCompSwap(g_head_ptr, current_head_ptr, current_head_ptr%g_particles.length());
		atomicAdd(g_workgroup_allocations, 1);
		group_head_ptr = allocated_head_ptr;
	}

	barrier();
	memoryBarrierShared();

	const uint particle_idx = (gl_LocalInvocationIndex + group_head_ptr) % g_particles.length();
	spawn_particle(particle_idx);
}