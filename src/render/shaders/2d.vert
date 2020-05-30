#version 450

layout(location = 0) in vec2 in_position;
layout(location = 1) in vec2 in_uv;

layout(location = 0) out VertexData {
    vec2 uv;
} o;

void main() {
    gl_Position = vec4(in_position, 0., 1.);
    o.uv = in_uv;
}
