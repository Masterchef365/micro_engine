#version 450

// Per-frame UBO
layout(binding = 0) uniform PerFrame {
    mat4 camera[2];
    uvec3 midi;
    float anim;
};

layout(location = 0) in vec3 frag_color;
layout(location = 0) out vec4 out_color;

void main() {
    out_color = vec4(vec3(midi) / 128., 1.0);
}
