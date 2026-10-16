// Hologram fragment shader
// Scan-line + flicker holographic effect for UI entities and overlays.

precision highp float;

uniform float uTime;
uniform float uOpacity;
uniform vec3 uColor;         // Base hologram color (default: cyan #00f0ff)
uniform float uScanSpeed;    // Scan line speed (default: 2.0)
uniform float uFlickerRate;  // Flicker frequency (default: 15.0)
uniform sampler2D uTexture;

varying vec2 vUv;

// Scan lines
float scanLine(float y, float time) {
    float line = sin(y * 300.0) * 0.5 + 0.5;
    float scan = sin(y * 40.0 - time * uScanSpeed) * 0.5 + 0.5;
    return mix(0.8, 1.0, line) * mix(0.9, 1.0, scan);
}

// Flicker
float flicker(float time) {
    float base = sin(time * uFlickerRate) * 0.02;
    float burst = step(0.98, fract(sin(time * 43.0) * 0.5 + 0.5)) * 0.15;
    return 1.0 - base - burst;
}

// Edge glow based on Fresnel-like effect (using UV distance from center)
float edgeGlow(vec2 uv) {
    vec2 center = vec2(0.5);
    float dist = length(uv - center);
    return smoothstep(0.3, 0.5, dist) * 0.4;
}

// Horizontal distortion glitch
vec2 glitch(vec2 uv, float time) {
    float glitchLine = step(0.995, fract(sin(floor(uv.y * 50.0) + time * 3.0) * 43758.5453));
    float offset = glitchLine * (sin(time * 100.0) * 0.02);
    return vec2(uv.x + offset, uv.y);
}

void main() {
    vec2 uv = glitch(vUv, uTime);

    vec4 texColor = texture2D(uTexture, uv);

    // Scan lines
    float scan = scanLine(vUv.y, uTime);

    // Flicker
    float flick = flicker(uTime);

    // Edge glow
    float edge = edgeGlow(vUv);

    // Compose hologram
    vec3 holo = uColor;

    // If texture has content, tint it
    if (texColor.a > 0.01) {
        holo = mix(uColor, texColor.rgb, 0.5);
    }

    // Apply effects
    holo *= scan * flick;

    // Add edge glow (additive)
    holo += uColor * edge;

    // Interference lines (thin horizontal bands)
    float interference = sin(vUv.y * 800.0 + uTime * 5.0) * 0.03;
    holo += vec3(interference);

    // Alpha: base opacity with scan modulation
    float alpha = uOpacity * scan * flick;

    // Fade at edges
    float edgeFade = smoothstep(0.0, 0.05, vUv.x) * smoothstep(1.0, 0.95, vUv.x)
                   * smoothstep(0.0, 0.05, vUv.y) * smoothstep(1.0, 0.95, vUv.y);
    alpha *= edgeFade;

    gl_FragColor = vec4(holo, alpha);
}
