/**
 * Eyeball.tsx — Three.js scene wrapper component using @react-three/fiber.
 *
 * Renders a high-fidelity eyeball with:
 *  - Sclera (white of the eye) with subtle veining
 *  - Procedural iris with radial fibres (canvas texture)
 *  - Animated pupil dilation driven by cursor distance
 *  - Corneal specular reflection
 *  - Smooth quaternion SLERP gaze tracking toward cursor
 *
 * @example
 * ```tsx
 * <Eyeball />
 * ```
 */
import React, { useRef, useMemo, useEffect, useState, Component, type ReactNode, type ErrorInfo } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import * as THREE from "three";
import { useEyeballTracking } from "./useEyeballTracking";

/* ── Configuration ─────────────────────────────────────────── */
const EYE_RADIUS = 1.6;
const IRIS_RADIUS = 0.7;
const SEGMENTS = 48;

/* ── Procedural Iris Texture Generator ─────────────────────── */
function createIrisTexture(
  baseColor: [number, number, number] = [34, 102, 180], // RGB blue
  pupilDilation: number = 0.3
): THREE.CanvasTexture {
  const size = 512;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d")!;

  const cx = size / 2;
  const cy = size / 2;
  const outerRadius = size / 2;
  const pupilRadius = outerRadius * pupilDilation;
  const irisInner = pupilRadius + 2;

  // Clear to transparent
  ctx.clearRect(0, 0, size, size);

  // Draw the iris ring with radial gradient
  const irisGrad = ctx.createRadialGradient(cx, cy, irisInner, cx, cy, outerRadius);
  const [r, g, b] = baseColor;
  // Keep colors saturated - don't add too much brightness near pupil
  irisGrad.addColorStop(0, `rgb(${Math.min(255, r + 15)}, ${Math.min(255, g + 20)}, ${Math.min(255, b + 10)})`); // slightly lighter near pupil
  irisGrad.addColorStop(0.2, `rgb(${r}, ${g}, ${b})`);
  irisGrad.addColorStop(0.5, `rgb(${Math.max(0, r - 10)}, ${Math.max(0, g - 5)}, ${Math.max(0, b - 5)})`);
  irisGrad.addColorStop(0.8, `rgb(${Math.max(0, r - 25)}, ${Math.max(0, g - 15)}, ${Math.max(0, b - 20)})`);
  irisGrad.addColorStop(1, `rgb(${Math.max(0, r - 40)}, ${Math.max(0, g - 30)}, ${Math.max(0, b - 30)})`); // dark limbal ring

  ctx.fillStyle = irisGrad;
  ctx.beginPath();
  ctx.arc(cx, cy, outerRadius, 0, Math.PI * 2);
  ctx.fill();

  // Draw radial fibres (the key to realistic iris)
  const numFibres = 120;
  for (let i = 0; i < numFibres; i++) {
    const angle = (Math.PI * 2 * i) / numFibres + (Math.random() - 0.5) * 0.1;
    const innerR = irisInner + Math.random() * 10;
    const outerR = outerRadius - 5 - Math.random() * 30;
    
    // Vary fiber color
    const colorShift = Math.random() * 40 - 20;
    const fiberAlpha = 0.15 + Math.random() * 0.2;
    ctx.strokeStyle = `rgba(${r + colorShift}, ${g + colorShift}, ${b + colorShift}, ${fiberAlpha})`;
    ctx.lineWidth = 1 + Math.random() * 2;
    
    ctx.beginPath();
    ctx.moveTo(cx + Math.cos(angle) * innerR, cy + Math.sin(angle) * innerR);
    
    // Add some waviness
    const midR = (innerR + outerR) / 2;
    const wobble = (Math.random() - 0.5) * 0.08;
    ctx.quadraticCurveTo(
      cx + Math.cos(angle + wobble) * midR,
      cy + Math.sin(angle + wobble) * midR,
      cx + Math.cos(angle) * outerR,
      cy + Math.sin(angle) * outerR
    );
    ctx.stroke();
  }

  // Draw darker radial spokes (crypts) - prominent black streaks
  const numCrypts = 48;
  for (let i = 0; i < numCrypts; i++) {
    const angle = (Math.PI * 2 * i) / numCrypts + (Math.random() - 0.5) * 0.25;
    const innerR = irisInner + 5;
    const outerR = outerRadius * (0.65 + Math.random() * 0.30);
    
    ctx.strokeStyle = `rgba(0, 0, 0, ${0.6 + Math.random() * 0.3})`;
    ctx.lineWidth = 2 + Math.random() * 3.5;
    
    ctx.beginPath();
    ctx.moveTo(cx + Math.cos(angle) * innerR, cy + Math.sin(angle) * innerR);
    // Add slight curve to the streak
    const midR = (innerR + outerR) / 2;
    const wobble = (Math.random() - 0.5) * 0.06;
    ctx.quadraticCurveTo(
      cx + Math.cos(angle + wobble) * midR,
      cy + Math.sin(angle + wobble) * midR,
      cx + Math.cos(angle) * outerR,
      cy + Math.sin(angle) * outerR
    );
    ctx.stroke();
  }

  // Additional fine black streaks radiating from pupil
  const numFineStreaks = 80;
  for (let i = 0; i < numFineStreaks; i++) {
    const angle = (Math.PI * 2 * i) / numFineStreaks + (Math.random() - 0.5) * 0.15;
    const innerR = irisInner + 2;
    const outerR = irisInner + (outerRadius - irisInner) * (0.3 + Math.random() * 0.6);
    
    ctx.strokeStyle = `rgba(0, 0, 0, ${0.4 + Math.random() * 0.3})`;
    ctx.lineWidth = 1 + Math.random() * 1.5;
    
    ctx.beginPath();
    ctx.moveTo(cx + Math.cos(angle) * innerR, cy + Math.sin(angle) * innerR);
    ctx.lineTo(cx + Math.cos(angle) * outerR, cy + Math.sin(angle) * outerR);
    ctx.stroke();
  }

  // Add subtle concentric rings (collarette)
  for (let ring = 0; ring < 3; ring++) {
    const ringR = irisInner + (outerRadius - irisInner) * (0.3 + ring * 0.2);
    ctx.strokeStyle = `rgba(0, 0, 0, ${0.05 + ring * 0.02})`;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(cx, cy, ringR, 0, Math.PI * 2);
    ctx.stroke();
  }

  // Dark limbal ring at outer edge
  ctx.strokeStyle = `rgba(30, 20, 10, 0.6)`;
  ctx.lineWidth = 8;
  ctx.beginPath();
  ctx.arc(cx, cy, outerRadius - 4, 0, Math.PI * 2);
  ctx.stroke();

  // BLACK PUPIL in center
  ctx.fillStyle = "#000000";
  ctx.beginPath();
  ctx.arc(cx, cy, pupilRadius, 0, Math.PI * 2);
  ctx.fill();

  // Pupil edge highlight (subtle)
  ctx.strokeStyle = `rgba(${r}, ${g}, ${b}, 0.3)`;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.arc(cx, cy, pupilRadius, 0, Math.PI * 2);
  ctx.stroke();

  const tex = new THREE.CanvasTexture(canvas);
  tex.needsUpdate = true;
  return tex;
}

/* ── Inner scene rendered inside the R3F Canvas ────────────── */
function EyeballScene({ gazeAngle, pupilDilation }: { gazeAngle: { x: number; y: number }; pupilDilation: number }) {
  const groupRef = useRef<THREE.Group>(null!);
  const irisMeshRef = useRef<THREE.Mesh>(null!);

  // ── Sclera (white of eye) ──────────────────────────────────
  const scleraMat = useMemo(
    () =>
      new THREE.MeshStandardMaterial({
        color: 0xf5f0eb,
        roughness: 0.35,
        metalness: 0.0,
        envMapIntensity: 0.4,
        transparent: true,
        opacity: 0.5,
      }),
    [],
  );

  // ── Blood vessel texture (procedural canvas) ───────────────
  const scleraMap = useMemo(() => {
    const size = 512;
    const canvas = document.createElement("canvas");
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext("2d")!;

    // Base white
    ctx.fillStyle = "#f5f0eb";
    ctx.fillRect(0, 0, size, size);

    // Subtle veins at edges
    ctx.strokeStyle = "rgba(180, 50, 50, 0.12)";
    ctx.lineWidth = 1.2;
    const cx = size / 2;
    const cy = size / 2;

    for (let i = 0; i < 18; i++) {
      const angle = (Math.PI * 2 * i) / 18 + Math.random() * 0.3;
      const r1 = size * 0.35;
      const r2 = size * 0.48;
      ctx.beginPath();
      ctx.moveTo(cx + Math.cos(angle) * r1, cy + Math.sin(angle) * r1);
      // Wiggly line outward
      for (let j = 0; j < 5; j++) {
        const t = (j + 1) / 5;
        const r = r1 + (r2 - r1) * t;
        const wiggle = (Math.random() - 0.5) * 12;
        ctx.lineTo(
          cx + Math.cos(angle + wiggle * 0.01) * r + wiggle,
          cy + Math.sin(angle + wiggle * 0.01) * r + wiggle,
        );
      }
      ctx.stroke();
    }

    // Dark fade from edges toward center (vignette effect) - lighter shadow
    const edgeFade = ctx.createRadialGradient(cx, cy, size * 0.15, cx, cy, size * 0.5);
    edgeFade.addColorStop(0, "rgba(0, 0, 0, 0)");      // transparent center
    edgeFade.addColorStop(0.3, "rgba(0, 0, 0, 0.03)"); // very slight darkening
    edgeFade.addColorStop(0.5, "rgba(0, 0, 0, 0.08)"); // light shadow
    edgeFade.addColorStop(0.7, "rgba(0, 0, 0, 0.18)"); // medium at edges
    edgeFade.addColorStop(0.9, "rgba(0, 0, 0, 0.3)");  // moderate shadow near rim
    edgeFade.addColorStop(1, "rgba(0, 0, 0, 0.45)");   // less dark at rim
    ctx.fillStyle = edgeFade;
    ctx.fillRect(0, 0, size, size);

    const tex = new THREE.CanvasTexture(canvas);
    tex.needsUpdate = true;
    return tex;
  }, []);

  // Apply sclera map
  useEffect(() => {
    scleraMat.map = scleraMap;
    scleraMat.needsUpdate = true;
  }, [scleraMat, scleraMap]);

  // ── Iris texture (regenerates when pupil dilates significantly) ──
  const irisTexture = useMemo(() => {
    // Brighter green iris
    return createIrisTexture([55, 180, 80], pupilDilation * 0.6); // Scale down dilation
  }, [Math.round(pupilDilation * 10) / 10]); // Only regenerate on significant changes

  const irisMat = useMemo(
    () =>
      new THREE.MeshBasicMaterial({
        map: irisTexture,
        transparent: true,
        opacity: 0.6,
        side: THREE.FrontSide,
      }),
    [irisTexture],
  );

  // ── Per-frame update: Apply gaze rotation ──────────────────
  useFrame(() => {
    if (groupRef.current) {
      // Instant tracking - no smoothing
      groupRef.current.rotation.x = gazeAngle.x;
      groupRef.current.rotation.y = gazeAngle.y;
    }
  });

  return (
    <group ref={groupRef}>
      {/* Sclera */}
      <mesh>
        <sphereGeometry args={[EYE_RADIUS, SEGMENTS, SEGMENTS]} />
        <primitive object={scleraMat} attach="material" />
      </mesh>

      {/* Iris with procedural texture (includes pupil) */}
      <mesh ref={irisMeshRef} position={[0, 0, EYE_RADIUS * 0.99]}>
        <circleGeometry args={[IRIS_RADIUS, SEGMENTS]} />
        <primitive object={irisMat} attach="material" />
      </mesh>

      {/* Corneal highlight - small spec */}
      <mesh position={[0.15, 0.2, EYE_RADIUS * 1.02]}>
        <circleGeometry args={[0.06, 16]} />
        <meshBasicMaterial color={0xffffff} transparent opacity={0.35} />
      </mesh>

      {/* Secondary smaller highlight */}
      <mesh position={[0.25, 0.1, EYE_RADIUS * 1.02]}>
        <circleGeometry args={[0.03, 12]} />
        <meshBasicMaterial color={0xffffff} transparent opacity={0.4} />
      </mesh>
    </group>
  );
}

/* ── Canvas Error Boundary ─────────────────────────────────── */
class CanvasErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) { return { error }; }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[Eyeball] Canvas error:", error, info);
  }
  render() {
    if (this.state.error) {
      return (
        <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center", color: "#ff4444", fontSize: 12, fontFamily: "monospace" }}>
          Eyeball error: {this.state.error.message}
        </div>
      );
    }
    return this.props.children;
  }
}

/* ── Inner Canvas Component (needs tracking in React context) ─ */
function EyeballCanvas() {
  const tracking = useEyeballTracking();
  
  return (
    <Canvas
      camera={{ position: [0, 0, 5], fov: 60, near: 0.1, far: 50 }}
      dpr={[1, 2]}
      frameloop="always"
      gl={{
        antialias: true,
        alpha: true,
        powerPreference: "high-performance",
      }}
      style={{ background: "transparent", position: "absolute", inset: 0, width: "100%", height: "100%" }}
    >
      {/* Lighting */}
      <ambientLight intensity={0.5} color={0xffffff} />
      <directionalLight position={[0, 0, 5]} intensity={0.9} color={0xffffff} />

      <EyeballScene gazeAngle={tracking.gazeAngle} pupilDilation={tracking.pupilDilation} />
    </Canvas>
  );
}

/* ── Public Component ──────────────────────────────────────── */
export interface EyeballProps {
  /** CSS class applied to the canvas container */
  className?: string;
}

/**
 * Interactive 3D eyeball that tracks the user's cursor.
 *
 * Renders inside an R3F Canvas with a perspective camera at 60° FOV.
 * All gaze tracking is computed via SLERP in `useEyeballTracking`.
 */
const Eyeball: React.FC<EyeballProps> = ({ className }) => {
  const [mounted, setMounted] = useState(false);
  useEffect(() => { setMounted(true); }, []);

  return (
    <div
      className={className}
      style={{
        width: "100%",
        height: "100%",
        position: "relative",
        minWidth: 200,
        minHeight: 200,
        // No filter - too expensive. Shadow handled by sclera texture.
      }}
      aria-hidden="true"
    >
      {mounted && (
        <CanvasErrorBoundary>
          <EyeballCanvas />
        </CanvasErrorBoundary>
      )}
    </div>
  );
};

export default Eyeball;
