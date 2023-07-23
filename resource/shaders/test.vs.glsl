
void main() {
	gl_Position = vec4(float(gl_VertexID - 5)/10.0, 0.0, 0.0, 1.0);
	gl_PointSize = 10.0;
}