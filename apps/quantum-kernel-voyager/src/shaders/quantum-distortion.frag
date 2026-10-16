// Quantum distortion fragment shader
// Used for warp tunnel and quantum-level entity effects.
// Applies sinusoidal distortion based on time uniform.

precision highp float;

uniform float uTime;
uniform float uIntensity;
uniform vec2 uResolution;
uniform sampler2D uTexture;

varying vec2 vUv;

const float PI = 3.14159265359;
const float TAU = 6.28318530718;

// Simplex-like hash for pseudo-random
vec2 hash(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)),
             dot(p, vec2(269.5, 183.3)));
    return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

// 2D noise
float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(dot(hash(i + vec2(0.0, 0.0)), f - vec2(0.0, 0.0)),
                   dot(hash(i + vec2(1.0, 0.0)), f - vec2(1.0, 0.0)), u.x),
               mix(dot(hash(i + vec2(0.0, 1.0)), f - vec2(0.0, 1.0)),
                   dot(hash(i + vec2(1.0, 1.0)), f - vec2(1.0, 1.0)), u.x), u.y);
}

void main() {
    vec2 uv = vUv;
    vec2 center = vec2(0.5);
    vec2 delta = uv - center;
    float dist = length(delta);

    // Radial distortion waves
    float wave = sin(dist * 20.0 - uTime * 4.0) * uIntensity * 0.02;
    float wave2 = sin(dist * 35.0 - uTime * 6.0) * uIntensity * 0.01;

    // Angular twist
    float angle = atan(delta.y, delta.x);
    float twist = sin(angle * 4.0 + uTime * 2.0) * uIntensity * 0.015;

    // Noise field displacement
    vec2 noiseUv = uv * 3.0 + uTime * 0.3;
    float n = noise(noiseUv) * uIntensity * 0.025;

    vec2 distortedUv = uv + normalize(delta + 0.001) * (wave + wave2) + vec2(twist, n);

    // Chromatic aberration by distortion intensity
    float aberration = uIntensity * 0.005;
    float r = texture2D(uTexture, distortedUv + vec2(aberration, 0.0)).r;
    float g = texture2D(uTexture, distortedUv).g;
    float b = texture2D(uTexture, distortedUv - vec2(aberration, 0.0)).b;

    // Vignette
    float vignette = 1.0 - smoothstep(0.3, 0.8, dist);

    vec3 color = vec3(r, g, b) * vignette;

    // Quantum energy tint (cyan → magenta radial)
    vec3 tint = mix(vec3(0.0, 0.94, 1.0), vec3(1.0, 0.0, 1.0), dist * 2.0);
    color += tint * uIntensity * 0.05 * (1.0 - dist);

    gl_FragColor = vec4(color, 1.0);
}
