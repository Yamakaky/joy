#version 450

struct Light {
    vec4 position;  
  
    vec3 ambient;
    vec3 diffuse;
    vec3 specular;
	
    float constant;
    float linear;
    float quadratic;
}; 

layout(location = 0) in VertexData {
    vec3 position;
    vec2 uv;
    float depth;
} i;

layout(location = 0) out vec4 out_color;
layout(location = 1) out float out_depth;

layout(set = 0, binding = 0)
#include "uniform.glsl"

layout(set = 1, binding = 0) uniform texture2D normals;
layout(set = 1, binding = 1) uniform sampler normals_sampler;

layout(set = 2, binding = 0, std140) uniform Lights {
    uint count;  
    Light item[10];
} lights;

void main() {
    vec3 normal_sample = texture(sampler2D(normals, normals_sampler), i.uv).xyz;
    vec3 normal = normalize(mat3(u.normal_transform) * normal_sample);

    vec3 lighting = vec3(0.);
    for (int idx = 0; idx < lights.count; idx++) {
        Light light = lights.item[idx];

        vec3 light_dir;
        float attenuation;
        if (light.position.w == 0.) {
            // Directional light
            light_dir = normalize(light.position.xyz);
            attenuation = 0.3;
        } else {
            // Point light
            vec3 light_vec = light.position.xyz / light.position.w - i.position;
            light_dir = normalize(light_vec);
            float distance = length(light_vec);
            attenuation = 1. / (
                1. + light.linear * distance + light.quadratic * (distance * distance)
            );  
        }

        float diffuse_strength  = max(dot(normal, light_dir), 0.);
        vec3 diffuse_color = light.diffuse * diffuse_strength;

        lighting += (light.ambient + diffuse_color) * attenuation;
    }
    out_color = vec4(1.) * vec4(lighting, 1.);
    out_depth = i.depth;
}
