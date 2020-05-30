#version 450

layout(location = 0) in VertexData {
    vec2 uv;
} i;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform texture2D ir_pixels;
layout(set = 0, binding = 1) uniform sampler ir_sampler;

void main() {
    float intensity = texture(sampler2D(ir_pixels, ir_sampler), i.uv).r;
    out_color = vec4(intensity, intensity, intensity, 1.);
}
