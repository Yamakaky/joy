#version 450

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_uv;

layout(location = 0) out vec2 f_uv;

layout(set = 0, binding = 0)
#include "uniform.glsl"

void main() {
    gl_Position = vec4(a_position, 0., 1.);
    f_uv = a_uv;
}
