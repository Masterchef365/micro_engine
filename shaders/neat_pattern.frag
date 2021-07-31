#version 450

// Per-frame UBO
layout(binding = 0) uniform PerFrame {
    mat4 camera[2];
    float anim;
};

layout(location = 0) in vec3 frag_color;
layout(location = 0) out vec4 out_color;

float wiggwle(float x) {
    return (cos(x) + 1.) / 2.;
}

vec3 hsv2rgb(vec3 c) {
  vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
  return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

void main() {
    vec2 st = frag_color.xy;// * 2. - 1.;

    bool b = true;
    float v = 0.;

    float z = wiggwle(anim / 1000.);
    float w = wiggwle(anim / 1320.);
    for (int i = 0; i < 19; i++) {
        b = b != fract((st.x + st.y) * float(i)) < z;
        b = b != fract((-st.x + st.y) * float(i)) < w;
        v += 0.05 * float(b);
    }
    //vec3 color = vec3(0.276,0.980,0.877) * v;
    vec3 color = hsv2rgb(vec3(v * 0.7 + 0.7, 1., v));

    out_color = vec4(color, 1.0);
}
