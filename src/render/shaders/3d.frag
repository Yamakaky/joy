#version 450

layout(location = 0) in vec3 f_normal;

layout(location = 0) out vec4 outColor;

const vec3 LIGHT_DIRECTION = vec3(0., 1., 1.);
const vec3 LIGHT_COLOR = vec3(1., 0., 0.);
const vec3 LIGHT_COLOR_AMBIENT = vec3(0., 0., 1.);
const float LIGHT_AMBIENT_INTENSITY = 0.1;

void main() {
    vec3 ambient_color = LIGHT_COLOR_AMBIENT * LIGHT_AMBIENT_INTENSITY;

    vec3 normal = normalize(f_normal);
    vec3 light_dir = normalize(LIGHT_DIRECTION);

    float diffuse_strength  = max(dot(normal, light_dir), 0.);
    vec3 diffuse_color = LIGHT_COLOR * diffuse_strength;

    outColor = vec4(1., 1., 1., 1.) * vec4(ambient_color + diffuse_color, 1.0);
}
