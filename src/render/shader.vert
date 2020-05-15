#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in uint a_depth;

layout(location = 0) out uint f_depth;

layout(set = 0, binding = 0)
uniform Uniforms {
    mat4 u_mvp;
    uint width;
    uint height;
};

void main() {
    vec3 pos = a_position + vec3(gl_InstanceIndex % width, gl_InstanceIndex / width, -float(255 - a_depth));
    gl_Position = u_mvp * vec4(height - pos.y, width - pos.x, pos.z, 1.0);
    f_depth = a_depth;
}
