#version 450

layout(location = 0) in VertexData {
    vec3 position;
    vec2 uv;
    float depth;
} i;

layout(location = 0) out vec4 out_color;
layout(location = 1) out float out_depth;

layout(set = 0, binding = 0)
#include "uniform.glsl"

layout(set = 1, binding = 0) uniform texture2D normals;
layout(set = 1, binding = 1) uniform sampler normals_sampler;

const vec4 LIGHT_POSITION = vec4(0., 0., 0., 1.0);
const vec3 LIGHT_COLOR = vec3(1., 0., 0.);
const vec3 LIGHT_COLOR_AMBIENT = vec3(0., 0., 1.);
const float LIGHT_AMBIENT_INTENSITY = 0.1;

void main() {
    if (!gl_FrontFacing) {
        out_color = vec4(0., 0.5, 0., 1.);
    } else {
        vec4 frag_color = vec4(1. - i.depth, 1. - i.depth, 1. - i.depth, 1.0);

        vec3 ambient_color = LIGHT_COLOR_AMBIENT * LIGHT_AMBIENT_INTENSITY;

        vec3 normal_sample = texture(sampler2D(normals, normals_sampler), i.uv).xyz;
        vec3 normal = normalize(mat3(transpose(inverse(u.ir_rotation))) * normal_sample);

        vec3 light_dir;
        if (LIGHT_POSITION.w == 0.) {
            // Directional light
            light_dir = normalize(LIGHT_POSITION.xyz);
        } else {
            // Point light
            light_dir = normalize(LIGHT_POSITION.xyz / LIGHT_POSITION.w - i.position);
        }

        float diffuse_strength  = max(dot(normal, light_dir), 0.);
        vec3 diffuse_color = LIGHT_COLOR * diffuse_strength;

        out_color = frag_color * vec4(ambient_color + diffuse_color, 1.0);
    }
    out_depth = i.depth;
}
