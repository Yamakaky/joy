#version 450

layout(location = 0) in VertexData {
    vec3 normal;
    float depth;
} i;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec4 out_depth;

const vec3 LIGHT_DIRECTION = vec3(0., -1., 1.);
const vec3 LIGHT_COLOR = vec3(1., 0., 0.);
const vec3 LIGHT_COLOR_AMBIENT = vec3(0., 0., 1.);
const float LIGHT_AMBIENT_INTENSITY = 0.1;

void main() {
    if (!gl_FrontFacing) {
        out_color = vec4(0., 0.5, 0., 1.);
    } else {
        vec4 frag_color = vec4(1. - i.depth, 1. - i.depth, 1. - i.depth, 1.0);

        vec3 ambient_color = LIGHT_COLOR_AMBIENT * LIGHT_AMBIENT_INTENSITY;

        vec3 normal = normalize(i.normal);
        vec3 light_dir = normalize(LIGHT_DIRECTION);

        float diffuse_strength  = max(dot(normal, -light_dir), 0.);
        vec3 diffuse_color = LIGHT_COLOR * diffuse_strength;

        out_color = frag_color * vec4(ambient_color + diffuse_color, 1.0);
    }
    out_depth.r = i.depth;
}
