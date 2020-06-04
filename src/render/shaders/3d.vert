#version 450

layout(location = 0) in vec4 in_position;
layout(location = 1) in vec4 in_normal;
layout(location = 2) in float in_depth;

layout(location = 0) out VertexData {
    vec3 position;
    vec3 normal;
    float depth;
} o;

layout(set = 0, binding = 0)
#include "uniform.glsl"

void main() {
    vec4 moved = u.ir_rotation * in_position;
    gl_Position = u.view_proj * moved;
    o.position = moved.xyz / moved.w;
    o.normal = mat3(transpose(inverse(u.ir_rotation))) * in_normal.xyz;
    o.depth = in_depth;
}
