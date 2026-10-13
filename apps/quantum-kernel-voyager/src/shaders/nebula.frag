// Nebula fragment shader
// Procedural volumetric nebula for skybox / background.
// Uses layered noise (fBM) with color gradients.

precision highp float;

uniform float uTime;
uniform vec2 uResolution;
uniform float uDensity;
uniform vec3 uColorA;    // Primary nebula color (default: cyan)
uniform vec3 uColorB;    // Secondary nebula color (default: magenta)
uniform vec3 uColorBg;   // Background color (default: dark blue)

varying vec2 vUv;

// Hash-based pseudo-random
float hash(vec3 p) {
    p = fract(p * 0.3183099 + 0.1);
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

// 3D noise
float noise3D(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(mix(hash(i + vec3(0, 0, 0)), hash(i + vec3(1, 0, 0)), f.x),
            mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), f.x), f.y),
        mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), f.x),
            mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), f.x), f.y),
        f.z);
}

// Fractional Brownian Motion (6 octaves)
float fbm(vec3 p) {
    float value = 0.0;
    float amplitude = 0.5;
    float frequency = 1.0;
    for (int i = 0; i < 6; i++) {
        value += amplitude * noise3D(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Domain warping for organic shapes
float warpedFbm(vec3 p) {
    vec3 q = vec3(
        fbm(p + vec3(0.0, 0.0, 0.0)),
        fbm(p + vec3(5.2, 1.3, 0.0)),
        fbm(p + vec3(0.0, 0.0, 3.7))
    );
    return fbm(p + q * 2.0);
}

void main() {
    vec2 uv = vUv * 2.0 - 1.0;
    uv.x *= uResolution.x / uResolution.y;

    // Construct 3D sample point (UV + time = moving through space)
    vec3 p = vec3(uv * 1.5, uTime * 0.05);

    // Layer 1: large-scale structure
    float n1 = warpedFbm(p * 0.5 + vec3(uTime * 0.02));

    // Layer 2: detail filaments
    float n2 = fbm(p * 2.0 + vec3(uTime * 0.03));

    // Layer 3: stars (high-frequency threshold)
    float stars = noise3D(p * 50.0);
    stars = smoothstep(0.97, 1.0, stars);

    // Combine
    float density = n1 * 0.6 + n2 * 0.4;
    density = smoothstep(0.3 - uDensity * 0.3, 0.7, density);

    // Color gradient based on density
    vec3 nebulaColor = mix(uColorA, uColorB, n2);
    vec3 color = mix(uColorBg, nebulaColor, density * 0.6);

    // Add self-illumination at high density
    float glow = smoothstep(0.5, 0.8, density) * 0.3;
    color += nebulaColor * glow;

    // Stars
    color += vec3(1.0) * stars * 0.8;

    // Subtle vignette
    float vignette = 1.0 - length(uv) * 0.3;
    color *= vignette;

    gl_FragColor = vec4(color, 1.0);
}
