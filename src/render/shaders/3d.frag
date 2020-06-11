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

const Light lights[2] = {Light(
    vec4(0., 0., 0., 1.0),
    vec3(0., 1., 0.) * 0.05,
    vec3(0., 1., 0.) * 0.8,
    vec3(0., 1., 0.),
    1.0,
    0.7,
    1.8
), Light(
    vec4(0.2, 1., -0.2, 0.0),
    vec3(0.05),
    vec3(0.4),
    vec3(0.5),
    1.0,
    0.7,
    1.8
)};

void main() {
    vec3 normal_sample = texture(sampler2D(normals, normals_sampler), i.uv).xyz;
    vec3 normal = normalize(mat3(transpose(inverse(u.ir_rotation))) * normal_sample);

    vec3 lighting = vec3(0.);
    for (int idx = 0; idx < 2; idx++) {
        Light light = lights[idx];

        vec3 light_dir;
        float attenuation;
        if (light.position.w == 0.) {
            // Directional light
            light_dir = normalize(light.position.xyz);
            attenuation = 0.3;
        } else {
            // Point light
            vec3 light_vec = light.position.xyz / light.position.w - i.position;
            light_dir = normalize(light.position.xyz / light.position.w - i.position);
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
