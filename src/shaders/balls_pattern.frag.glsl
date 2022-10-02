#version 450


layout(std140, row_major, binding = 1) uniform PatternData {
	vec2 screen_dimensions;
	float time;
};



flat in uint v_pattern_index;
flat in uint v_shape_id;
in vec4 v_color_0;
in vec4 v_color_1;

layout(location=0) out vec4 out_color;


float circle(vec2 position, float radius) {
	return length(position) - radius;
}

float rectangle(vec2 position, vec2 halfSize) {
    vec2 d = abs(position)-halfSize;
    return length(max(d,0.0)) + min(max(d.x,d.y),0.0);
}

vec2 translate(vec2 position, vec2 offset) {
	return position - offset;
}

vec2 rotate(vec2 position, float rotation){
	const float PI = 3.14159;
	float angle = -rotation * PI * 2.0;
	float s = sin(angle);
	float c = cos(angle);
	return vec2(
		c * position.x + s * position.y,
		c * position.y - s * position.x
	);
}

vec2 mirror(vec2 position) {
	return abs(position);
}

float merge(float shape1, float shape2){
	return min(shape1, shape2);
}

float intersect(float shape1, float shape2){
	return max(shape1, shape2);
}

float subtract(float base, float subtraction){
	return intersect(base, -subtraction);
}

vec2 cells(inout vec2 position, vec2 period){
    vec2 cell_index = position / period;
    cell_index = floor(cell_index);

    position = mod(position, period);
    //negative positions lead to negative modulo
    position += period;
    //negative positions now have correct cell coordinates, positive input positions too high
    position = mod(position, period);
    //second mod doesn't change values between 0 and period, but brings down values that are above period.

    return cell_index;
}


float scene(vec2 position) {
	position *= 100.0;

	vec2 period = vec2(1.5);
	vec2 cell_idx = cells(position, period);
	vec2 flip = abs(mod(cell_idx, 2.0));
	position = mix(position, period - position, flip);

	float r = rectangle(rotate(position, 0.125 + time*0.1), vec2(0.7071));

	// r = subtract(abs(r) - 0.1, r);

	float c = circle(position, 1.0);

	float combined = mix(r, c, sin(time*2.0 + floor(0.5 + cell_idx.x/2.0) + floor(0.5 + cell_idx.y/2.0))*0.5 + 0.5);
	combined = merge(abs(combined) - 0.1, abs(combined+0.4) - 0.1);
	// combined = mod(combined, 2.0);

	return -combined;
}



void scroll_columns(inout vec2 position, float column_width) {
	float column = floor(position.x / column_width);
	position.y -= (mod(column, 2.0) * 2.0 - 1.0) * time * 3.0;
}

void stagger_columns(inout vec2 position, float column_width) {
	float column = floor(position.x / column_width);
	position.y -= mod(column, 2.0);
}




float stripes(vec2 position, vec2 direction) {
	position *= 100.0;

	float dist = dot(position, direction);
	return mod(dist, 2.0) - 1.0;
}

float squares(vec2 position) {
	position *= 100.0;
	cells(position, vec2(2.0));
	return rectangle(position, vec2(1.0));
}


float diamonds(vec2 position) {
	position *= 100.0;

	// Repeat
	position = mod(position, vec2(3.0)) - vec2(1.5);


	position = rotate(position, 1.0/8.0);

	return rectangle(position, vec2(0.8));
}


float crosses(vec2 position) {
	position *= 50.0;

	// Scroll
	position += vec2(time * 3.0);

	stagger_columns(position, 3.0);

	// Repeat
	position = mod(position, vec2(3.0)) - vec2(1.5);

	position = rotate(position, 1.0/8.0);

	float rect_0 = rectangle(position, vec2(1.0, 0.4));
	float rect_1 = rectangle(position, vec2(0.4, 1.0));

	return merge(rect_0, rect_1);
}

float rand(vec2 n) { 
	return fract(sin(dot(n, vec2(12.9898, 4.1414))) * 43758.5453);
}

float noise(vec2 p){
	vec2 ip = floor(p);
	vec2 u = fract(p);
	u = u*u*(3.0-2.0*u);
	
	float res = mix(
		mix(rand(ip),rand(ip+vec2(1.0,0.0)),u.x),
		mix(rand(ip+vec2(0.0,1.0)),rand(ip+vec2(1.0,1.0)),u.x),u.y);
	return res*res;
}

float noise_pattern(vec2 position) {
	vec2 shape_offset = vec2(float(v_shape_id));
	float ns = noise(position * 2.0 + vec2(time/4.0) - shape_offset)
		+ 0.3 * noise(position * 4.0 - vec2(time/3.0) + shape_offset);

	float nssmooth = smoothstep(0.3, 0.7, ns);

	return step(nssmooth, noise(floor(position * screen_dimensions.y)));
}


void main() {
	vec2 uv = (gl_FragCoord.xy - screen_dimensions/2.0 ) / screen_dimensions.y;

	if (v_pattern_index == 3) {
		float value = uv.y + 0.5;

		vec3 color = mix(v_color_0.rgb, v_color_1.rgb, smoothstep(0.1, 0.9, value));
		out_color = vec4(color, 1.0);
		return;
	}
	if (v_pattern_index == 4) {
		float value = noise_pattern(uv);

		vec3 color = mix(v_color_0.rgb, v_color_1.rgb, float(value));
		out_color = vec4(color, 1.0);
		return;
	}

	float dist = 0.0;

	switch(v_pattern_index) {
		case 0: dist = stripes(uv, vec2(0.0, 1.0)); break;
		case 1: dist = squares(uv); break;
		case 2: dist = crosses(uv); break;
		// case 4: dist = noise(uv); break;
	}

	float dist_dx = fwidth(dist) * 0.5;
	float value = smoothstep(dist_dx, -dist_dx, dist);

	vec3 color = mix(v_color_0.rgb, v_color_1.rgb, float(value));
	out_color = vec4(color, 1.0);
}