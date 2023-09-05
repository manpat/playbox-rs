layout(local_size_x=1) in;


// must be std140 so it can be bound as UBO
layout(binding=0, std140) writeonly buffer Output {
	vec2 s_points[3];
};

void main() {
	s_points[0] = vec2(-0.5, -0.5);
	s_points[1] = vec2( 0.0,  0.5);
	s_points[2] = vec2( 0.5, -0.5);
}