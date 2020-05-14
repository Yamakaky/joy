#version 450

layout(location=0) in vec2 a_position;
layout(location=1) in float a_instance_offset;

void main() {
    gl_Position = vec4(a_position.x + a_instance_offset, a_position.y + a_instance_offset, 0.0, 1.0);
}
