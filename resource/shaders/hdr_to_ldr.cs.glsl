layout(local_size_x=8, local_size_y=8) in;

layout(binding=0, rgba16f) readonly uniform image2D u_hdr_image;
layout(binding=1, rgba8) writeonly uniform image2D u_ldr_image;


layout(binding = 0, std430) readonly buffer A {
	float u_dither_time;
	float u_dither_quantise;

	float u_tonemap_contrast;
	float u_tonemap_exposure;

	uint u_tonemap_algorithm;
};


// https://registry.khronos.org/DataFormat/specs/1.3/dataformat.1.3.html#TRANSFER_SRGB
vec3 linear_to_gamma(vec3 linear) {
	vec3 selector = ceil(linear - 0.0031308);
	vec3 less_than_branch = linear * 12.92;
	vec3 greater_than_branch = 1.055 * pow(linear, vec3(1.0/2.4)) - 0.055;
	return mix(less_than_branch, greater_than_branch, selector);
}

// https://gist.github.com/patriciogonzalezvivo/670c22f3966e662d2f83
float hash(float n) { return fract(sin(n) * 1e4); }
float hash(vec2 p) { return fract(1e4 * sin(17.0 * p.x + p.y * 0.1) * (0.1 + abs(sin(p.y * 13.0 + p.x)))); }


vec3 saturate(vec3 v) {
	return clamp(v, 0.0, 1.0);
}

vec3 tonemap_reinhardt(vec3 color) {
	return color / (color + vec3(1.0));
}

vec3 tonemap_aces_filmic(vec3 x) {
	float a = 2.51;
	float b = 0.03;
	float c = 2.43;
	float d = 0.59;
	float e = 0.14;
	return saturate((x*(a*x+b))/(x*(c*x+d)+e));
}

void main() {
	ivec2 texel_uv = ivec2(gl_GlobalInvocationID.xy);

	ivec2 image_size = imageSize(u_hdr_image);
	if (any(greaterThanEqual(texel_uv, image_size))) {
		return;
	}

	vec4 hdr_color = imageLoad(u_hdr_image, texel_uv);

	vec3 color = hdr_color.rgb;

	const vec3 middle_grey = vec3(0.5);

	// Apply contrast
	vec3 log_color = log(color);
	log_color = (log_color - middle_grey) * u_tonemap_contrast + middle_grey;
	color = exp(log_color);

	color = max(color, 0.0) * u_tonemap_exposure;

	// Tonemap
	switch (u_tonemap_algorithm) {
	case 1: 
		color = tonemap_reinhardt(color);
		break;
	case 2: 
		color = tonemap_aces_filmic(color);
		break;
	default: break; 
	};

	// linear_to_gamma required because u_ldr_image is in an srgb format, so we have to do the conversion ourselves
	color = saturate(color);
	color = linear_to_gamma(color);

	// Dither and quantize
	float noise = hash(vec2(texel_uv) / vec2(image_size) + fract(u_dither_time/1000000.0) - 0.5) * 2.0;
	color = round(color*u_dither_quantise + noise - 0.5)/u_dither_quantise;

	imageStore(u_ldr_image, texel_uv, vec4(color, 1.0));
}