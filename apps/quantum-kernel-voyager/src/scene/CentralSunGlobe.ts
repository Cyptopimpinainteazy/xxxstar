/**
 * @module scene/CentralSunGlobe
 * Golden sun-like sphere with animated molten gold river/vein channels,
 * wireframe lattice overlay, and Fresnel rim glow.
 *
 * Cyberpunk wireframe style — neon-gold emissive on dark background.
 */
import * as THREE from "three";

// ---------------------------------------------------------------------------
// GLSL shaders (inlined to avoid build-config changes)
// ---------------------------------------------------------------------------

const VERT = /* glsl */ `
varying vec2 vUv;
varying vec3 vNormal;
varying vec3 vPosition;

void main() {
  vUv = uv;
  vNormal = normalize(normalMatrix * normal);
  vPosition = (modelViewMatrix * vec4(position, 1.0)).xyz;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

const FRAG = /* glsl */ `
uniform float uTime;
uniform vec3  baseColor;
uniform vec3  channelColor;
uniform float emissiveIntensity;
uniform float fresnelPower;
uniform float wireOpacity;
uniform float veinStrength;

varying vec2 vUv;
varying vec3 vNormal;
varying vec3 vPosition;

/* ---- hash noise ---- */
float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
/* ---- FBM ---- */
float fbm(vec2 p) {
  float v = 0.0, a = 0.5;
  mat2 rot = mat2(0.8, 0.6, -0.6, 0.8);
  for (int i = 0; i < 4; i++) {
    v += a * noise(p);
    p = rot * p * 2.0;
    a *= 0.5;
  }
  return v;
}
/* ---- Fresnel ---- */
float fresnel(vec3 viewDir, vec3 norm, float pw) {
  return pow(1.0 - max(dot(normalize(viewDir), normalize(norm)), 0.0), pw);
}
/* ---- Procedural molten veins ---- */
float veinPattern(vec2 uv, float t) {
  vec2 flowA = uv * 8.0 + vec2(t * 0.15, t * 0.08);
  float va = fbm(flowA);
  vec2 flowB = uv * 12.0 + vec2(-t * 0.1, t * 0.12);
  float vb = fbm(flowB);
  float c = va * 0.7 + vb * 0.3;
  float ch = smoothstep(0.35, 0.55, c);
  float core = smoothstep(0.45, 0.52, c) * 1.5;
  return ch + core * 0.3;
}
/* ---- Wireframe grid ---- */
float wireframe(vec2 uv, float w) {
  vec2 g = abs(fract(uv * vec2(16.0, 8.0) - 0.5) - 0.5);
  float l = min(g.x, g.y);
  return 1.0 - smoothstep(0.0, w, l);
}

void main() {
  vec3 viewDir = -normalize(vPosition);
  float fr = fresnel(viewDir, vNormal, fresnelPower);
  float veins = veinPattern(vUv, uTime);

  // Base + channel blend (veins subdued for dark glassy core)
  vec3 color = mix(baseColor, channelColor, veins * veinStrength);

  // Pulsing glow in channels
  float pulse = sin(uTime * 1.5) * 0.15 + 0.85;
  color += channelColor * veins * 0.25 * pulse;

  // Fresnel rim glow (gold → white)
  vec3 rimColor = mix(baseColor * 1.5, vec3(1.0, 0.95, 0.8), fr);
  color += rimColor * emissiveIntensity * fr;

  // White-hot corona fringe
  color = mix(color, vec3(1.0, 0.98, 0.9), fr * fr * 0.4);

  // Wireframe lattice
  float w = wireframe(vUv, 0.018);
  color = mix(color, channelColor * 1.4, w * wireOpacity);

  // Emissive boost
  color *= 1.0 + emissiveIntensity * 0.2;

  gl_FragColor = vec4(color, 1.0);
}
`;

// ---------------------------------------------------------------------------
// Class
// ---------------------------------------------------------------------------

export interface CentralSunGlobeConfig {
  /** Sphere radius. Default = 12 (matches SUN_RADIUS). */
  radius?: number;
  /** Segment count (higher = smoother). Default = 64. */
  segments?: number;
  /** Wireframe lattice line opacity. Default = 0.7. */
  wireOpacity?: number;
  /** Show the glowing logo mark on the sphere surface. Default = true. */
  showLogo?: boolean;
  /** Logo text (e.g., "X3"). Default = "X3". */
  logoText?: string;
}

export class CentralSunGlobe {
  readonly group: THREE.Group;

  private readonly material: THREE.ShaderMaterial;
  private readonly wireframeMesh: THREE.LineSegments;

  constructor(config: CentralSunGlobeConfig = {}) {
    const radius = config.radius ?? 12;
    const segments = config.segments ?? 64;
    const wireOpacity = config.wireOpacity ?? 0.95;
    const showLogo = config.showLogo ?? true;
    const logoText = config.logoText ?? "X3";

    this.group = new THREE.Group();
    this.group.name = "atlas-central-sun";

    // -- Shader sphere (molten gold veins + Fresnel) --
    this.material = new THREE.ShaderMaterial({
      uniforms: {
        uTime: { value: 0 },
        baseColor: { value: new THREE.Color("#0a0a12") },
        channelColor: { value: new THREE.Color("#d8a54a") },
        emissiveIntensity: { value: 0.9 },
        fresnelPower: { value: 2.5 },
        wireOpacity: { value: wireOpacity },
        veinStrength: { value: 0.45 },
      },
      vertexShader: VERT,
      fragmentShader: FRAG,
      transparent: false,
      side: THREE.FrontSide,
      depthWrite: true,
    });

    const sphereGeo = new THREE.SphereGeometry(radius, segments, segments);
    const sphere = new THREE.Mesh(sphereGeo, this.material);
    sphere.name = "sun-sphere";
    this.group.add(sphere);

    // -- Wireframe lattice overlay (slightly larger for cyberpunk edge pop) --
    const wireGeo = new THREE.SphereGeometry(radius * 1.005, segments / 2, segments / 2);
    const edges = new THREE.EdgesGeometry(wireGeo);
    const wireMat = new THREE.LineBasicMaterial({
      color: 0xffd700,
      transparent: true,
      opacity: 0.6,
      linewidth: 1,
    });
    this.wireframeMesh = new THREE.LineSegments(edges, wireMat);
    this.wireframeMesh.name = "sun-wireframe";
    this.group.add(this.wireframeMesh);

    // -- Outer glow backface sphere --
    const glowGeo = new THREE.SphereGeometry(radius * 1.25, 32, 32);
    const glowMat = new THREE.MeshBasicMaterial({
      color: 0xffc266,
      transparent: true,
      opacity: 0.12,
      side: THREE.BackSide,
    });
    const glow = new THREE.Mesh(glowGeo, glowMat);
    glow.name = "sun-glow";
    this.group.add(glow);

    if (showLogo) {
      const logoTexture = createLogoTexture(logoText);
      logoTexture.anisotropy = 4;
      logoTexture.colorSpace = THREE.SRGBColorSpace;

      const logoMat = new THREE.MeshBasicMaterial({
        map: logoTexture,
        transparent: true,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      });

      const logoSize = radius * 1.05;
      const logoGeo = new THREE.PlaneGeometry(logoSize, logoSize);
      const logo = new THREE.Mesh(logoGeo, logoMat);
      logo.position.set(0, 0, radius * 1.01);
      logo.name = "sun-logo";
      this.group.add(logo);
    }

    // Clean up intermediate geos (wireGeo used for edge extraction)
    wireGeo.dispose();
  }

  /** Call every frame with delta time. */
  update(dt: number): void {
    this.material.uniforms.uTime.value += dt * 0.5;

    // Slow self-rotation for visual interest
    this.group.rotation.y += dt * 0.02;
  }

  dispose(): void {
    this.group.traverse((child) => {
      if (child instanceof THREE.Mesh || child instanceof THREE.LineSegments) {
        child.geometry?.dispose();
        if (child.material instanceof THREE.Material) {
          child.material.dispose();
        }
      }
    });
  }
}

function createLogoTexture(text: string): THREE.CanvasTexture {
  const size = 512;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");

  if (ctx) {
    ctx.clearRect(0, 0, size, size);

    // Base glow
    const gradient = ctx.createRadialGradient(
      size * 0.5,
      size * 0.5,
      size * 0.05,
      size * 0.5,
      size * 0.5,
      size * 0.5,
    );
    gradient.addColorStop(0, "rgba(255, 215, 0, 0.9)");
    gradient.addColorStop(1, "rgba(255, 215, 0, 0.0)");
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, size, size);

    // Text glow
    ctx.font = "bold 240px Arial";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.shadowColor = "rgba(255, 210, 120, 0.9)";
    ctx.shadowBlur = 40;
    ctx.fillStyle = "rgba(255, 210, 120, 1.0)";
    ctx.fillText(text, size * 0.5, size * 0.52);

    // Crisp inner stroke
    ctx.shadowBlur = 0;
    ctx.lineWidth = 6;
    ctx.strokeStyle = "rgba(255, 255, 255, 0.9)";
    ctx.strokeText(text, size * 0.5, size * 0.52);
  }

  const tex = new THREE.CanvasTexture(canvas);
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.generateMipmaps = false;
  return tex;
}
