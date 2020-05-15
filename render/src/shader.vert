#version 450

layout(location = 0) in vec2 a_position;
layout(location = 1) in uint a_depth;

layout(location = 0) out float f_color;

layout(set = 0, binding = 0)
uniform Uniforms {
    mat4 u_mvp;
    uint width;
    uint height;
};

void main() {
    vec2 off = vec2(gl_InstanceIndex % width, gl_InstanceIndex / width);
    gl_Position = u_mvp * vec4(a_position.xy + off, a_depth, 1.0);
    f_color = a_depth / 256.;
}
