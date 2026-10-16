/**
 * Shader-Based Animated Backgrounds
 * 
 * WebGL2/WebGPU shader-driven animated backgrounds for Atlas Sphere UI.
 * Supports multiple visual styles with smooth transitions.
 * 
 * Styles:
 * - Neon Grid: Pulsing cyber grid with bloom
 * - Quantum Grid: Probabilistic wave interference patterns
 * - Liquid Metal: Fluid dynamics simulation
 * - Procedural Fractals: Infinite zoom fractals
 * - Neural Network: Synaptic connection visualization
 */

'use client';

import React, { useEffect, useRef, useCallback, useState } from 'react';

// =============================================================================
// Types
// =============================================================================

export type BackgroundStyle = 
  | 'neon-grid' 
  | 'quantum-grid' 
  | 'liquid-metal' 
  | 'procedural-fractal'
  | 'neural-network'
  | 'particle-field'
  | 'aurora'
  | 'matrix-rain';

export interface ShaderBackgroundProps {
  /** Visual style to render */
  style: BackgroundStyle;
  /** Primary color (CSS color or hex) */
  primaryColor?: string;
  /** Secondary color */
  secondaryColor?: string;
  /** Accent color for highlights */
  accentColor?: string;
  /** Animation speed multiplier */
  speed?: number;
  /** Intensity of the effect */
  intensity?: number;
  /** Opacity of the background */
  opacity?: number;
  /** Enable mouse interactivity */
  interactive?: boolean;
  /** Quality preset */
  quality?: 'low' | 'medium' | 'high';
  /** CSS class name */
  className?: string;
  /** Enable reduced motion for accessibility */
  reducedMotion?: boolean;
}

// =============================================================================
// Shader Sources
// =============================================================================

const VERTEX_SHADER = `#version 300 es
precision highp float;

in vec2 a_position;
out vec2 v_uv;

void main() {
  v_uv = a_position * 0.5 + 0.5;
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

const NEON_GRID_SHADER = `#version 300 es
precision highp float;

uniform float u_time;
uniform vec2 u_resolution;
uniform vec2 u_mouse;
uniform vec3 u_primaryColor;
uniform vec3 u_secondaryColor;
uniform vec3 u_accentColor;
uniform float u_speed;
uniform float u_intensity;

in vec2 v_uv;
out vec4 fragColor;

float grid(vec2 uv, float lineWidth) {
  vec2 grid = abs(fract(uv - 0.5) - 0.5) / fwidth(uv);
  float line = min(grid.x, grid.y);
  return 1.0 - min(line, 1.0);
}

void main() {
  vec2 uv = v_uv;
  float aspect = u_resolution.x / u_resolution.y;
  uv.x *= aspect;
  
  float time = u_time * u_speed;
  
  // Perspective grid
  vec2 gridUV = uv * 10.0;
  gridUV.y += time * 0.5;
  
  // Horizontal scan line
  float scanLine = smoothstep(0.0, 0.1, abs(sin(uv.y * 50.0 + time * 2.0))) * 0.1;
  
  // Grid lines
  float gridPattern = grid(gridUV, 0.02);
  float gridGlow = grid(gridUV, 0.1) * 0.5;
  
  // Pulsing nodes at intersections
  vec2 nodeUV = fract(gridUV);
  float nodeDist = length(nodeUV - 0.5);
  float node = smoothstep(0.15, 0.05, nodeDist);
  float nodePulse = node * (0.5 + 0.5 * sin(time * 3.0 + length(floor(gridUV)) * 0.5));
  
  // Color mixing
  vec3 gridColor = mix(u_primaryColor, u_secondaryColor, gridPattern);
  vec3 nodeColor = u_accentColor * nodePulse * 2.0;
  
  // Mouse interaction (glow at cursor)
  vec2 mouseUV = u_mouse;
  mouseUV.x *= aspect;
  float mouseDist = length(uv - mouseUV);
  float mouseGlow = exp(-mouseDist * 5.0) * u_intensity;
  
  vec3 finalColor = gridColor * (gridPattern + gridGlow) * u_intensity;
  finalColor += nodeColor;
  finalColor += u_accentColor * mouseGlow * 0.5;
  finalColor += u_primaryColor * scanLine;
  
  // Bloom effect
  finalColor += finalColor * 0.3;
  
  fragColor = vec4(finalColor, 1.0);
}
`;

const QUANTUM_GRID_SHADER = `#version 300 es
precision highp float;

uniform float u_time;
uniform vec2 u_resolution;
uniform vec2 u_mouse;
uniform vec3 u_primaryColor;
uniform vec3 u_secondaryColor;
uniform vec3 u_accentColor;
uniform float u_speed;
uniform float u_intensity;

in vec2 v_uv;
out vec4 fragColor;

// Quantum wave function visualization
float wave(vec2 p, float freq, float time) {
  return sin(p.x * freq + time) * cos(p.y * freq * 0.7 + time * 0.8);
}

float interference(vec2 uv, float time) {
  float w1 = wave(uv, 8.0, time);
  float w2 = wave(uv + vec2(0.3, 0.5), 12.0, time * 1.3);
  float w3 = wave(uv - vec2(0.2, 0.4), 6.0, time * 0.7);
  return (w1 + w2 + w3) / 3.0;
}

// Probability density visualization
float probability(vec2 uv, float time) {
  float psi = interference(uv, time);
  return psi * psi; // |ψ|²
}

void main() {
  vec2 uv = v_uv * 2.0 - 1.0;
  float aspect = u_resolution.x / u_resolution.y;
  uv.x *= aspect;
  
  float time = u_time * u_speed;
  
  // Quantum probability field
  float prob = probability(uv, time);
  
  // Energy levels (quantized bands)
  float energy = fract(prob * 5.0);
  float band = floor(prob * 5.0) / 5.0;
  
  // Superposition visualization
  float superpos1 = probability(uv + vec2(0.1 * sin(time), 0.1 * cos(time)), time);
  float superpos2 = probability(uv - vec2(0.1 * cos(time), 0.1 * sin(time)), time * 1.1);
  
  // Entanglement lines
  vec2 entangled = uv + vec2(prob * 0.1, -prob * 0.1);
  float entanglement = abs(sin(length(entangled) * 20.0 + time * 2.0));
  
  // Color based on probability density
  vec3 lowEnergy = u_primaryColor;
  vec3 highEnergy = u_secondaryColor;
  vec3 peak = u_accentColor;
  
  vec3 color = mix(lowEnergy, highEnergy, prob);
  color = mix(color, peak, smoothstep(0.7, 1.0, prob));
  
  // Add interference pattern
  color *= 0.8 + 0.2 * energy;
  
  // Entanglement glow
  color += u_accentColor * entanglement * 0.2;
  
  // Uncertainty principle visualization (blur at edges)
  float uncertainty = smoothstep(0.0, 0.3, length(uv - u_mouse * 2.0 + 1.0));
  color *= uncertainty * 0.5 + 0.5;
  
  color *= u_intensity;
  
  fragColor = vec4(color, 1.0);
}
`;

const LIQUID_METAL_SHADER = `#version 300 es
precision highp float;

uniform float u_time;
uniform vec2 u_resolution;
uniform vec2 u_mouse;
uniform vec3 u_primaryColor;
uniform vec3 u_secondaryColor;
uniform vec3 u_accentColor;
uniform float u_speed;
uniform float u_intensity;

in vec2 v_uv;
out vec4 fragColor;

// Simplex noise
vec3 mod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec2 mod289(vec2 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec3 permute(vec3 x) { return mod289(((x*34.0)+1.0)*x); }

float snoise(vec2 v) {
  const vec4 C = vec4(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
  vec2 i  = floor(v + dot(v, C.yy));
  vec2 x0 = v - i + dot(i, C.xx);
  vec2 i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
  vec4 x12 = x0.xyxy + C.xxzz;
  x12.xy -= i1;
  i = mod289(i);
  vec3 p = permute(permute(i.y + vec3(0.0, i1.y, 1.0)) + i.x + vec3(0.0, i1.x, 1.0));
  vec3 m = max(0.5 - vec3(dot(x0,x0), dot(x12.xy,x12.xy), dot(x12.zw,x12.zw)), 0.0);
  m = m*m; m = m*m;
  vec3 x = 2.0 * fract(p * C.www) - 1.0;
  vec3 h = abs(x) - 0.5;
  vec3 ox = floor(x + 0.5);
  vec3 a0 = x - ox;
  m *= 1.79284291400159 - 0.85373472095314 * (a0*a0 + h*h);
  vec3 g;
  g.x = a0.x * x0.x + h.x * x0.y;
  g.yz = a0.yz * x12.xz + h.yz * x12.yw;
  return 130.0 * dot(m, g);
}

// Fluid dynamics simulation (simplified)
float fluid(vec2 uv, float time) {
  float n1 = snoise(uv * 2.0 + time * 0.3);
  float n2 = snoise(uv * 4.0 - time * 0.5);
  float n3 = snoise(uv * 8.0 + time * 0.2);
  return n1 * 0.5 + n2 * 0.3 + n3 * 0.2;
}

// Metallic reflection
float fresnel(float cosTheta, float f0) {
  return f0 + (1.0 - f0) * pow(1.0 - cosTheta, 5.0);
}

void main() {
  vec2 uv = v_uv;
  float aspect = u_resolution.x / u_resolution.y;
  uv.x *= aspect;
  
  float time = u_time * u_speed;
  
  // Fluid displacement
  float displacement = fluid(uv, time);
  vec2 displaced = uv + displacement * 0.1;
  
  // Surface normal from fluid
  float dx = fluid(displaced + vec2(0.01, 0.0), time) - fluid(displaced - vec2(0.01, 0.0), time);
  float dy = fluid(displaced + vec2(0.0, 0.01), time) - fluid(displaced - vec2(0.0, 0.01), time);
  vec3 normal = normalize(vec3(-dx * 5.0, -dy * 5.0, 1.0));
  
  // View and light directions
  vec3 viewDir = normalize(vec3(uv - 0.5, 1.0));
  vec3 lightDir = normalize(vec3(0.5, 0.5, 1.0));
  vec3 halfDir = normalize(lightDir + viewDir);
  
  // Metallic reflection
  float NdotH = max(dot(normal, halfDir), 0.0);
  float NdotV = max(dot(normal, viewDir), 0.0);
  float NdotL = max(dot(normal, lightDir), 0.0);
  
  float spec = pow(NdotH, 64.0);
  float fres = fresnel(NdotV, 0.8);
  
  // Base metallic color
  vec3 metalColor = mix(u_primaryColor, u_secondaryColor, displacement * 0.5 + 0.5);
  
  // Iridescence
  float irid = sin(displacement * 10.0 + time) * 0.5 + 0.5;
  vec3 iridColor = mix(u_secondaryColor, u_accentColor, irid);
  
  // Final color
  vec3 color = metalColor * NdotL * 0.5;
  color += spec * u_accentColor * 2.0;
  color = mix(color, iridColor, fres * 0.5);
  
  // Mouse ripple
  float mouseDist = length(uv - u_mouse * vec2(aspect, 1.0));
  float ripple = sin(mouseDist * 30.0 - time * 5.0) * exp(-mouseDist * 3.0);
  color += u_accentColor * ripple * 0.3;
  
  color *= u_intensity;
  
  fragColor = vec4(color, 1.0);
}
`;

const FRACTAL_SHADER = `#version 300 es
precision highp float;

uniform float u_time;
uniform vec2 u_resolution;
uniform vec2 u_mouse;
uniform vec3 u_primaryColor;
uniform vec3 u_secondaryColor;
uniform vec3 u_accentColor;
uniform float u_speed;
uniform float u_intensity;

in vec2 v_uv;
out vec4 fragColor;

// Complex number multiplication
vec2 cmul(vec2 a, vec2 b) {
  return vec2(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

// Mandelbrot iteration
float mandelbrot(vec2 c, int maxIter) {
  vec2 z = vec2(0.0);
  int i;
  for (i = 0; i < maxIter; i++) {
    z = cmul(z, z) + c;
    if (dot(z, z) > 4.0) break;
  }
  if (i == maxIter) return 0.0;
  
  // Smooth iteration count
  float log_zn = log(dot(z, z)) / 2.0;
  float nu = log(log_zn / log(2.0)) / log(2.0);
  return float(i) + 1.0 - nu;
}

// Julia set
float julia(vec2 z, vec2 c, int maxIter) {
  int i;
  for (i = 0; i < maxIter; i++) {
    z = cmul(z, z) + c;
    if (dot(z, z) > 4.0) break;
  }
  if (i == maxIter) return 0.0;
  
  float log_zn = log(dot(z, z)) / 2.0;
  float nu = log(log_zn / log(2.0)) / log(2.0);
  return float(i) + 1.0 - nu;
}

void main() {
  vec2 uv = v_uv * 2.0 - 1.0;
  float aspect = u_resolution.x / u_resolution.y;
  uv.x *= aspect;
  
  float time = u_time * u_speed * 0.1;
  
  // Continuous zoom
  float zoom = 1.0 + time * 0.5;
  zoom = pow(2.0, mod(zoom, 8.0) - 4.0);
  
  // Zoom center (interesting Mandelbrot coordinate)
  vec2 center = vec2(-0.743643887037151, 0.131825904205330);
  
  // Apply zoom
  vec2 c = uv / zoom + center;
  
  // Animated Julia set parameter
  vec2 juliaC = vec2(
    -0.7 + 0.1 * sin(time * 0.3),
    0.27 + 0.1 * cos(time * 0.4)
  );
  
  // Choose between Mandelbrot and Julia based on position
  float blend = smoothstep(-0.5, 0.5, sin(time * 0.2));
  
  float m = mandelbrot(c, 100);
  float j = julia(uv * 1.5, juliaC, 100);
  
  float iter = mix(m, j, blend);
  
  // Color palette
  float t = iter / 50.0;
  vec3 color;
  
  if (iter == 0.0) {
    color = vec3(0.0);
  } else {
    // Psychedelic coloring
    color = 0.5 + 0.5 * cos(6.28318 * (t + vec3(0.0, 0.33, 0.67)));
    color = mix(u_primaryColor, color, 0.7);
    color = mix(color, u_accentColor, pow(t, 2.0));
  }
  
  // Edge glow
  float edge = fwidth(iter) * 10.0;
  color += u_secondaryColor * edge;
  
  color *= u_intensity;
  
  fragColor = vec4(color, 1.0);
}
`;

const NEURAL_NETWORK_SHADER = `#version 300 es
precision highp float;

uniform float u_time;
uniform vec2 u_resolution;
uniform vec2 u_mouse;
uniform vec3 u_primaryColor;
uniform vec3 u_secondaryColor;
uniform vec3 u_accentColor;
uniform float u_speed;
uniform float u_intensity;

in vec2 v_uv;
out vec4 fragColor;

// Hash function for pseudo-random positions
float hash(float n) {
  return fract(sin(n) * 43758.5453123);
}

vec2 hash2(vec2 p) {
  return vec2(hash(dot(p, vec2(127.1, 311.7))), hash(dot(p, vec2(269.5, 183.3))));
}

// Neural node positions
vec2 nodePos(int layer, int node, float time) {
  float layerX = float(layer) / 4.0 - 0.5;
  float nodeY = (float(node) / 8.0 - 0.5) * 0.8;
  
  // Slight animation
  float offset = hash(float(layer * 100 + node));
  nodeY += sin(time + offset * 6.28) * 0.02;
  
  return vec2(layerX, nodeY);
}

// Draw connection between two points
float connection(vec2 uv, vec2 a, vec2 b, float width, float activation) {
  vec2 ab = b - a;
  vec2 ap = uv - a;
  float t = clamp(dot(ap, ab) / dot(ab, ab), 0.0, 1.0);
  vec2 closest = a + t * ab;
  float dist = length(uv - closest);
  
  // Pulse along connection
  float pulse = sin(t * 20.0 - activation * 10.0) * 0.5 + 0.5;
  
  return smoothstep(width, width * 0.1, dist) * pulse;
}

// Draw node
float node(vec2 uv, vec2 pos, float radius, float activation) {
  float dist = length(uv - pos);
  float core = smoothstep(radius, radius * 0.1, dist);
  float glow = smoothstep(radius * 3.0, radius, dist) * 0.5;
  float pulse = 0.5 + 0.5 * sin(activation * 10.0);
  return (core + glow) * pulse;
}

void main() {
  vec2 uv = v_uv * 2.0 - 1.0;
  float aspect = u_resolution.x / u_resolution.y;
  uv.x *= aspect;
  
  float time = u_time * u_speed;
  
  vec3 color = vec3(0.0);
  
  // Draw connections
  float connectionSum = 0.0;
  for (int layer = 0; layer < 4; layer++) {
    for (int srcNode = 0; srcNode < 8; srcNode++) {
      vec2 src = nodePos(layer, srcNode, time);
      
      for (int dstNode = 0; dstNode < 8; dstNode++) {
        vec2 dst = nodePos(layer + 1, dstNode, time);
        
        // Weight-like activation
        float weight = hash(float(layer * 1000 + srcNode * 100 + dstNode));
        float activation = sin(time * 2.0 + weight * 6.28) * 0.5 + 0.5;
        
        if (weight > 0.3) { // Only draw some connections
          connectionSum += connection(uv, src, dst, 0.005, time + weight);
        }
      }
    }
  }
  
  color += u_primaryColor * connectionSum * 0.5;
  
  // Draw nodes
  float nodeSum = 0.0;
  for (int layer = 0; layer < 5; layer++) {
    for (int n = 0; n < 8; n++) {
      vec2 pos = nodePos(layer, n, time);
      float activation = sin(time * 3.0 + float(layer + n)) * 0.5 + 0.5;
      nodeSum += node(uv, pos, 0.03, time + float(layer));
    }
  }
  
  color += u_secondaryColor * nodeSum * 0.8;
  color += u_accentColor * nodeSum * nodeSum;
  
  // Signal propagation effect
  float signal = sin(uv.x * 10.0 + time * 5.0) * 0.5 + 0.5;
  color += u_accentColor * signal * 0.1 * smoothstep(1.0, 0.5, abs(uv.y));
  
  color *= u_intensity;
  
  fragColor = vec4(color, 1.0);
}
`;

// Shader map
const SHADERS: Record<BackgroundStyle, string> = {
  'neon-grid': NEON_GRID_SHADER,
  'quantum-grid': QUANTUM_GRID_SHADER,
  'liquid-metal': LIQUID_METAL_SHADER,
  'procedural-fractal': FRACTAL_SHADER,
  'neural-network': NEURAL_NETWORK_SHADER,
  'particle-field': NEON_GRID_SHADER, // Fallback
  'aurora': QUANTUM_GRID_SHADER, // Fallback
  'matrix-rain': NEON_GRID_SHADER, // Fallback
};

// =============================================================================
// Color Utilities
// =============================================================================

function hexToRgb(hex: string): [number, number, number] {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (result) {
    return [
      parseInt(result[1], 16) / 255,
      parseInt(result[2], 16) / 255,
      parseInt(result[3], 16) / 255,
    ];
  }
  return [1, 1, 1];
}

function cssColorToRgb(color: string): [number, number, number] {
  if (color.startsWith('#')) {
    return hexToRgb(color);
  }
  // Handle CSS color names, rgb(), etc. by creating a temp element
  if (typeof document !== 'undefined') {
    const temp = document.createElement('div');
    temp.style.color = color;
    document.body.appendChild(temp);
    const computed = getComputedStyle(temp).color;
    document.body.removeChild(temp);
    const match = computed.match(/\d+/g);
    if (match && match.length >= 3) {
      return [
        parseInt(match[0]) / 255,
        parseInt(match[1]) / 255,
        parseInt(match[2]) / 255,
      ];
    }
  }
  return [1, 1, 1];
}

// =============================================================================
// Component
// =============================================================================

export const ShaderBackground: React.FC<ShaderBackgroundProps> = ({
  style,
  primaryColor = '#00ffff',
  secondaryColor = '#ff00ff',
  accentColor = '#ffff00',
  speed = 1.0,
  intensity = 1.0,
  opacity = 1.0,
  interactive = true,
  quality = 'high',
  className = '',
  reducedMotion = false,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const glRef = useRef<WebGL2RenderingContext | null>(null);
  const programRef = useRef<WebGLProgram | null>(null);
  const animationRef = useRef<number>(0);
  const mouseRef = useRef<[number, number]>([0.5, 0.5]);
  
  const [isInitialized, setIsInitialized] = useState(false);

  // Initialize WebGL
  const initGL = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return false;

    const gl = canvas.getContext('webgl2', {
      alpha: true,
      premultipliedAlpha: false,
      antialias: quality === 'high',
      powerPreference: quality === 'low' ? 'low-power' : 'high-performance',
    });

    if (!gl) {
      console.error('WebGL2 not supported');
      return false;
    }

    glRef.current = gl;

    // Compile vertex shader
    const vertShader = gl.createShader(gl.VERTEX_SHADER);
    if (!vertShader) return false;
    gl.shaderSource(vertShader, VERTEX_SHADER);
    gl.compileShader(vertShader);

    if (!gl.getShaderParameter(vertShader, gl.COMPILE_STATUS)) {
      console.error('Vertex shader error:', gl.getShaderInfoLog(vertShader));
      return false;
    }

    // Compile fragment shader
    const fragShader = gl.createShader(gl.FRAGMENT_SHADER);
    if (!fragShader) return false;
    gl.shaderSource(fragShader, SHADERS[style]);
    gl.compileShader(fragShader);

    if (!gl.getShaderParameter(fragShader, gl.COMPILE_STATUS)) {
      console.error('Fragment shader error:', gl.getShaderInfoLog(fragShader));
      return false;
    }

    // Create program
    const program = gl.createProgram();
    if (!program) return false;
    gl.attachShader(program, vertShader);
    gl.attachShader(program, fragShader);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Program link error:', gl.getProgramInfoLog(program));
      return false;
    }

    programRef.current = program;

    // Create fullscreen quad
    const positions = new Float32Array([
      -1, -1,
       1, -1,
      -1,  1,
       1,  1,
    ]);

    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);

    const posLoc = gl.getAttribLocation(program, 'a_position');
    gl.enableVertexAttribArray(posLoc);
    gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);

    return true;
  }, [style, quality]);

  // Render frame
  const render = useCallback((time: number) => {
    const gl = glRef.current;
    const program = programRef.current;
    const canvas = canvasRef.current;

    if (!gl || !program || !canvas) {
      animationRef.current = requestAnimationFrame(render);
      return;
    }

    // Handle resize
    if (canvas.width !== canvas.clientWidth || canvas.height !== canvas.clientHeight) {
      canvas.width = canvas.clientWidth * (quality === 'low' ? 0.5 : 1);
      canvas.height = canvas.clientHeight * (quality === 'low' ? 0.5 : 1);
      gl.viewport(0, 0, canvas.width, canvas.height);
    }

    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(program);

    // Set uniforms
    const effectiveSpeed = reducedMotion ? 0.1 : speed;
    
    gl.uniform1f(gl.getUniformLocation(program, 'u_time'), time * 0.001 * effectiveSpeed);
    gl.uniform2f(gl.getUniformLocation(program, 'u_resolution'), canvas.width, canvas.height);
    gl.uniform2f(gl.getUniformLocation(program, 'u_mouse'), mouseRef.current[0], mouseRef.current[1]);
    gl.uniform3fv(gl.getUniformLocation(program, 'u_primaryColor'), cssColorToRgb(primaryColor));
    gl.uniform3fv(gl.getUniformLocation(program, 'u_secondaryColor'), cssColorToRgb(secondaryColor));
    gl.uniform3fv(gl.getUniformLocation(program, 'u_accentColor'), cssColorToRgb(accentColor));
    gl.uniform1f(gl.getUniformLocation(program, 'u_speed'), effectiveSpeed);
    gl.uniform1f(gl.getUniformLocation(program, 'u_intensity'), intensity);

    // Draw
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);

    animationRef.current = requestAnimationFrame(render);
  }, [primaryColor, secondaryColor, accentColor, speed, intensity, quality, reducedMotion]);

  // Mouse tracking
  useEffect(() => {
    if (!interactive) return;

    const handleMouseMove = (e: MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      
      const rect = canvas.getBoundingClientRect();
      mouseRef.current = [
        (e.clientX - rect.left) / rect.width,
        1 - (e.clientY - rect.top) / rect.height,
      ];
    };

    window.addEventListener('mousemove', handleMouseMove);
    return () => window.removeEventListener('mousemove', handleMouseMove);
  }, [interactive]);

  // Initialize and start rendering
  useEffect(() => {
    const success = initGL();
    setIsInitialized(success);

    if (success) {
      animationRef.current = requestAnimationFrame(render);
    }

    return () => {
      cancelAnimationFrame(animationRef.current);
    };
  }, [initGL, render]);

  // Re-initialize on style change
  useEffect(() => {
    if (isInitialized) {
      cancelAnimationFrame(animationRef.current);
      const success = initGL();
      if (success) {
        animationRef.current = requestAnimationFrame(render);
      }
    }
  }, [style, initGL, render, isInitialized]);

  return (
    <canvas
      ref={canvasRef}
      className={`shader-background shader-background--${style} ${className}`}
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        zIndex: -1,
        opacity,
        pointerEvents: interactive ? 'auto' : 'none',
      }}
      aria-hidden="true"
    />
  );
};

export default ShaderBackground;
