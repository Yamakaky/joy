#version 450

layout(location = 0) in vec2 f_uv;

layout(location = 0) out vec4 outColor;

layout(set = 1, binding = 0) uniform texture2D ir_pixels;
layout(set = 1, binding = 1) uniform sampler ir_sampler;

void main() {
    float intensity = texture(sampler2D(ir_pixels, ir_sampler), f_uv).r;
    outColor = vec4(intensity, intensity, intensity, 1.);
}
