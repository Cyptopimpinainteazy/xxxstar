/**
 * Gold flow fragment shader — procedural molten gold vein pattern
 * with Fresnel rim glow and time-based animation.
 *
 * Uniforms:
 *   uTime            — elapsed time for animation
 *   baseColor        — deep gold base (#FFD700)
 *   channelColor     — bright molten channel color (#FFF8DC)
 *   emissiveIntensity— overall glow strength
 *   fresnelPower     — edge glow exponent (higher = thinner rim)
 *   wireOpacity      — opacity of the wireframe lattice lines
 */

uniform float uTime;
uniform vec3 baseColor;
uniform vec3 channelColor;
uniform float emissiveIntensity;
uniform float fresnelPower;
uniform float wireOpacity;

varying vec2 vUv;
varying vec3 vNormal;
varying vec3 vPosition;
varying vec3 vWorldPosition;

// ---- Simplex-like hash noise for organic vein patterns ----
float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f); // smoothstep
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// Fractal Brownian Motion for layered detail
float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  mat2 rot = mat2(0.8, 0.6, -0.6, 0.8);
  for (int i = 0; i < 4; i++) {
    v += a * noise(p);
    p = rot * p * 2.0;
    a *= 0.5;
  }
  return v;
}

// Fresnel: bright at glancing angles
float fresnel(vec3 viewDir, vec3 normal, float power) {
  return pow(1.0 - max(dot(normalize(viewDir), normalize(normal)), 0.0), power);
}

// Procedural gold river/vein channels
float veinPattern(vec2 uv, float time) {
  // Primary flowing channels — scroll UV over time
  vec2 flowUV = uv * 8.0 + vec2(time * 0.15, time * 0.08);
  float veins = fbm(flowUV);

  // Secondary cross-veins for complexity
  vec2 crossUV = uv * 12.0 + vec2(-time * 0.1, time * 0.12);
  float cross = fbm(crossUV);

  // Combine into river-like channels
  float combined = veins * 0.7 + cross * 0.3;

  // Threshold to create distinct channel paths
  float channels = smoothstep(0.35, 0.55, combined);

  // Add bright core to channels
  float core = smoothstep(0.45, 0.52, combined) * 1.5;

  return channels + core * 0.3;
}

// Wireframe lattice lines (procedural grid on sphere surface)
float wireframe(vec2 uv, float lineWidth) {
  vec2 grid = abs(fract(uv * vec2(16.0, 8.0) - 0.5) - 0.5);
  float line = min(grid.x, grid.y);
  return 1.0 - smoothstep(0.0, lineWidth, line);
}

void main() {
  vec3 viewDir = -normalize(vPosition);

  // Fresnel rim glow
  float fresnelTerm = fresnel(viewDir, vNormal, fresnelPower);

  // Procedural veins
  float veins = veinPattern(vUv, uTime);

  // Base sphere color with vein highlights
  vec3 color = mix(baseColor, channelColor, veins * 0.8);

  // Add pulsing emissive glow to channels
  float pulse = sin(uTime * 1.5) * 0.15 + 0.85;
  color += channelColor * veins * 0.4 * pulse;

  // Fresnel rim glow — gold to white at edges
  vec3 rimColor = mix(baseColor * 1.5, vec3(1.0, 0.95, 0.8), fresnelTerm);
  color += rimColor * emissiveIntensity * fresnelTerm;

  // White-hot tint on strong Fresnel (sun corona feel)
  color = mix(color, vec3(1.0, 0.98, 0.9), fresnelTerm * fresnelTerm * 0.4);

  // Wireframe lattice overlay
  float wire = wireframe(vUv, 0.02);
  vec3 wireColor = baseColor * 1.8; // bright gold wireframe
  color = mix(color, wireColor, wire * wireOpacity);

  // Overall emissive boost
  color *= 1.0 + emissiveIntensity * 0.2;

  // Slight HDR bloom contribution (bright areas > 1.0 will bloom)
  gl_FragColor = vec4(color, 1.0);
}
