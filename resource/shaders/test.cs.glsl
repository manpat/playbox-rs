layout(local_size_x=1) in;


layout(binding=0) writeonly buffer Output {
	vec2 s_points[3];
};

void main() {
	s_points[0] = vec2(-0.5, -0.5);
	s_points[1] = vec2( 0.0,  0.5);
	s_points[2] = vec2( 0.5, -0.5);
}