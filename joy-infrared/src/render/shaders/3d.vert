#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in float in_depth;

layout(location = 0) out VertexData {
    vec3 position;
    vec2 uv;
    float depth;
} o;

layout(set = 0, binding = 0)
#include "uniform.glsl"

void main() {
    vec4 moved = u.ir_rotation * vec4(in_position, 1.0);
    gl_Position = u.view_proj * moved;
    o.position = moved.xyz / moved.w;
    o.uv = in_uv;
    o.depth = in_depth;
}
