#version 450
#import global
#import gbuffer_particle

out vec4 v_color;
out uint v_particle_id;


void main() {
	uint particle_id = gl_VertexID; // TODO: should possible be sparse?
	Particle particle = g_particles[particle_id];

	gl_Position = u_projection_view * vec4(particle.position, 1.0);
	gl_PointSize = 15.0;

	v_color = particle.color;
	v_particle_id = particle_id;
}