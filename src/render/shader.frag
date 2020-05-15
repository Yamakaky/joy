#version 450

layout(location = 0) flat in uint f_depth;

layout(location = 0) out vec4 outColor;

void main() {
    if (f_depth < 5) {

    outColor = vec4(1.0, 0.0, 0.0, 0.0);
    } else {

    outColor = vec4(0.0, 1.0, 0.0, 1.0);
    }
}
