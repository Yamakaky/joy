#version 450

layout(location = 0) flat in uint f_depth;

layout(location = 0) out vec4 outColor;

void main() {
    float factor = float(f_depth) / 256.;
    outColor = vec4(1. - factor, factor, 0.0, 1.0);
}
