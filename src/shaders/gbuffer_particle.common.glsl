
struct Particle {
	vec3 position;
	vec3 velocity;
	vec4 color;
};


layout(std430, binding = 0) buffer ParticleData {
    Particle g_particles[];
};


layout(std430, binding = 1) buffer ControlData {
    uint g_head_ptr;
    uint g_workgroup_allocations;
};

