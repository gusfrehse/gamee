#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

void main() {
    float zoom = 0.40;
    vec2 pos = vec2(-0.0, -0.0);
    vec2 norm_coordinates = (gl_GlobalInvocationID.xy / vec2(imageSize(img)) - vec2(0.5)) / zoom;
    vec2 c = norm_coordinates + pos;

    vec2 z = vec2(0.0);
    float i;

    for (i = 0.0; i < 1.0; i += 0.005) {
	z = vec2(
	    z.x * z.x - z.y * z.y + c.x,
	    z.x * z.y + z.x * z.y + c.y
	    );
	// z = vec2(
	//     abs(z.x) * abs(z.x) - abs(z.y) * abs(z.y) + c.x,
	//     abs(z.x) * abs(z.y) + abs(z.x) * abs(z.y) + c.y
	//     );

	if (length(z) > 1/zoom) {
	    break;
	}
    }

    float E = 2.7182818284;
    //vec4 to_write = vec4(vec3(1/(1- E) + exp(i)/(E - 1)), 1.0);
    vec4 to_write = vec4(vec3(1.0 - i), 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}
    
