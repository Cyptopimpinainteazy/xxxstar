/**
 * GLSL shaders for advanced eyeball effects.
 *
 * The iris shader uses procedural radial fibre patterns and Fresnel rim
 * lighting to create a realistic, slightly luminous iris appearance.
 */

// ── Iris vertex shader ────────────────────────────────────────
export const irisVertexShader = /* glsl */ `
  varying vec2 vUv;
  varying vec3 vNormal;
  varying vec3 vViewDir;

  void main() {
    vUv = uv;
    vNormal = normalize(normalMatrix * normal);
    vec4 worldPos = modelViewMatrix * vec4(position, 1.0);
    vViewDir = normalize(-worldPos.xyz);
    gl_Position = projectionMatrix * worldPos;
  }
`;

// ── Iris fragment shader ──────────────────────────────────────
export const irisFragmentShader = /* glsl */ `
  uniform vec3 uIrisColor;
  uniform float uTime;
  uniform float uDilation;
  uniform float uBrightness;
  uniform vec2 uIrisOffset;

  varying vec2 vUv;
  varying vec3 vNormal;
  varying vec3 vViewDir;

  void main() {
    vec2 uv = vUv - 0.5;
    float dist = length(uv);

    // PUPIL: black center (small)
    float pupilRadius = 0.1;
    
    // IRIS: colored area fills most of the circle
    float irisOuter = 0.5;
    
    // Radial streaks for iris texture
    float angle = atan(uv.y, uv.x);
    float streaks = 0.7 + 0.3 * sin(angle * 24.0 + dist * 5.0);
    
    // Iris color with streaks
    vec3 irisCol = uIrisColor * streaks;
    
    // Darken toward the outer edge (limbal ring)
    float limbalDark = smoothstep(0.3, 0.5, dist);
    irisCol *= (1.0 - limbalDark * 0.6);
    
    // Start with iris color everywhere
    vec3 col = irisCol;
    
    // Black pupil in center
    float pupilMask = smoothstep(pupilRadius + 0.02, pupilRadius - 0.02, dist);
    col = mix(col, vec3(0.0), pupilMask);
    
    // Alpha: fully opaque inside the circle
    float alpha = smoothstep(irisOuter + 0.01, irisOuter - 0.01, dist);

    gl_FragColor = vec4(col, alpha);
  }
`;

// ── Corneal highlight vertex shader ───────────────────────────
export const cornealVertexShader = /* glsl */ `
  varying vec3 vNormal;
  varying vec3 vViewDir;

  void main() {
    vNormal = normalize(normalMatrix * normal);
    vec4 worldPos = modelViewMatrix * vec4(position, 1.0);
    vViewDir = normalize(-worldPos.xyz);
    gl_Position = projectionMatrix * worldPos;
  }
`;

// ── Corneal highlight fragment shader ─────────────────────────
export const cornealFragmentShader = /* glsl */ `
  uniform vec3 uLightDir;

  varying vec3 vNormal;
  varying vec3 vViewDir;

  void main() {
    // Tight specular highlight
    vec3 halfDir = normalize(uLightDir + vViewDir);
    float spec = pow(max(dot(vNormal, halfDir), 0.0), 512.0);

    // Very small, subtle highlight
    float alpha = spec * 0.3;
    gl_FragColor = vec4(vec3(1.0), alpha);
  }
`;
