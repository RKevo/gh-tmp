shader_type spatial;
render_mode unshaded;

uniform vec3 color_one : source_color; 
uniform vec3 color_two : source_color;
uniform vec3 color_outline : source_color;  

uniform float angle = 20.0;
uniform float line_count = 20.0; 
uniform float speed = 10.0; 
uniform float blur : hint_range(0.0, 2.0, 0); 
uniform vec2 TEXTURE_PIXEL_SIZE = vec2(64.0, 64.0);

varying vec3 world_normal;

vec2 rotate(vec2 uv, float rotation_angle) {
    float radians_angle = radians(rotation_angle);
    float cos_angle = cos(radians_angle);
    float sin_angle = sin(radians_angle);
    mat2 rotation_matrix = mat2(vec2(cos_angle, -sin_angle), vec2(sin_angle, cos_angle));
    return uv * rotation_matrix;
}

float stripe(vec2 uv, float scale) {
    return cos(uv.x * 0.0 - TIME*speed + uv.y * -line_count/2.0);
}

void vertex() {	
	world_normal = (MODEL_MATRIX*vec4(NORMAL, 0.0)).xyz;
	//world_normal = vec3(world_normal.x, world_normal.z, -world_normal.y);
}

void fragment() {
    vec2 uv = SCREEN_UV;
	uv = uv* 10.0;
    vec2 resolution = 1.0 / TEXTURE_PIXEL_SIZE;
    float a = TEXTURE_PIXEL_SIZE.x / TEXTURE_PIXEL_SIZE.y;
    uv.x *= a;
    uv = rotate(uv, angle);
    float g = stripe(uv, length(NODE_POSITION_VIEW));
    vec3 col = mix(color_one, color_two, smoothstep(0.0, blur, g));
	if (dot(world_normal, NODE_POSITION_WORLD - CAMERA_POSITION_WORLD) > 1.0) {
		ALBEDO = color_outline;
	} else {
    ALBEDO = vec3(col);
	}
}
