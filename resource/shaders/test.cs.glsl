layout(local_size_x=1) in;


// must be std140 so it can be bound as UBO
layout(binding=0, std140) writeonly buffer VertexOutput {
	vec2 s_points[3];
};

layout(binding=1) writeonly buffer IndexOutput {
	uint s_indices[6];
};


void main() {
	s_points[0] = vec2(-0.5, -0.5);
	s_points[1] = vec2( 0.0,  0.5);
	s_points[2] = vec2( 0.5, -0.5);

	s_indices[0] = 0;
	s_indices[1] = 1;
	s_indices[2] = 1;
	s_indices[3] = 2;
	s_indices[4] = 2;
	s_indices[5] = 3;
}