/**
 * @module scene/Animations
 * Warp transitions, entity state animations, and flow visualizations.
 *
 * - Warp: 2-second stretch + flash + arrival
 * - Entity bob/spin/pulse based on entity kind
 * - Data flow tube particle advancement
 */
import * as THREE from "three";
import type { SceneEntityKind } from "../types/scene";

// -------------------------------------------------------------------------
// Easing functions
// -------------------------------------------------------------------------

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

function easeOutExpo(t: number): number {
  return t === 1 ? 1 : 1 - Math.pow(2, -10 * t);
}

// -------------------------------------------------------------------------
// Warp transition
// -------------------------------------------------------------------------

export interface WarpState {
  active: boolean;
  elapsed: number;
  duration: number;
  phase: "stretch" | "flash" | "arrive";
  fromPos: THREE.Vector3;
  toPos: THREE.Vector3;
  callback?: () => void;
}

export function createWarpState(): WarpState {
  return {
    active: false,
    elapsed: 0,
    duration: 2.0,
    phase: "stretch",
    fromPos: new THREE.Vector3(),
    toPos: new THREE.Vector3(),
  };
}

/**
 * Start a warp transition.
 * @param state Warp state object (mutated)
 * @param from Starting position
 * @param to Destination position
 * @param onComplete Optional callback on arrival
 */
export function startWarp(
  state: WarpState,
  from: THREE.Vector3,
  to: THREE.Vector3,
  onComplete?: () => void,
): void {
  state.active = true;
  state.elapsed = 0;
  state.phase = "stretch";
  state.fromPos.copy(from);
  state.toPos.copy(to);
  state.callback = onComplete;
}

/**
 * Update warp each frame. Returns the current interpolated position.
 * Apply the returned position to the ship.
 */
export function updateWarp(state: WarpState, dt: number): THREE.Vector3 | null {
  if (!state.active) return null;

  state.elapsed += dt;
  const t = Math.min(state.elapsed / state.duration, 1);

  if (t < 0.3) {
    // Stretch phase: tunnel effect
    state.phase = "stretch";
    const st = t / 0.3;
    const eased = easeInOutCubic(st);
    return new THREE.Vector3().lerpVectors(state.fromPos, state.toPos, eased * 0.1);
  } else if (t < 0.5) {
    // Flash phase: white flash, rapid transit
    state.phase = "flash";
    const ft = (t - 0.3) / 0.2;
    const eased = easeOutExpo(ft);
    return new THREE.Vector3().lerpVectors(state.fromPos, state.toPos, 0.1 + eased * 0.8);
  } else {
    // Arrival phase: decelerate into destination
    state.phase = "arrive";
    const at = (t - 0.5) / 0.5;
    const eased = easeInOutCubic(at);
    const pos = new THREE.Vector3().lerpVectors(state.fromPos, state.toPos, 0.9 + eased * 0.1);

    if (t >= 1) {
      state.active = false;
      state.callback?.();
    }
    return pos;
  }
}

// -------------------------------------------------------------------------
// Entity idle animations (per-frame)
// -------------------------------------------------------------------------

/**
 * Apply idle animation to an entity's Object3D.
 * Call every frame with the current elapsed time.
 */
export function animateEntity(
  object: THREE.Object3D,
  kind: SceneEntityKind,
  time: number,
  _dt: number,
): void {
  switch (kind) {
    case "blockchain_node":
      // Slow rotation + gentle bob
      object.rotation.y = time * 0.3;
      object.position.y += Math.sin(time * 1.5) * 0.003;
      break;

    case "data_flow":
      // Advance flow particles along curve
      animateDataFlowParticles(object, time);
      break;

    case "contract":
      // Tumble rotation
      object.rotation.x = time * 0.2;
      object.rotation.y = time * 0.35;
      break;

    case "vault":
      // Pulse shield opacity + ring rotation
      object.children.forEach((child) => {
        if (child instanceof THREE.Mesh && child.material instanceof THREE.MeshBasicMaterial && child.geometry instanceof THREE.IcosahedronGeometry) {
          child.material.opacity = 0.15 + Math.sin(time * 2) * 0.1;
        }
        if (child instanceof THREE.Mesh && child.geometry instanceof THREE.TorusGeometry) {
          child.rotation.z = time * 0.5;
        }
      });
      break;

    case "planet":
      // Slow axial rotation
      object.rotation.y = time * 0.05;
      break;

    case "voyager_ship":
      // Gentle nose oscillation
      object.rotation.z = Math.sin(time * 0.8) * 0.02;
      object.rotation.x = Math.sin(time * 0.5) * 0.01;
      break;

    case "artifact":
      // Float + spin
      object.rotation.y = time * 0.8;
      object.position.y += Math.sin(time * 2) * 0.005;
      break;

    case "waypoint":
      // Beacon pulse
      object.children.forEach((child) => {
        if (child instanceof THREE.PointLight) {
          child.intensity = 1.5 + Math.sin(time * 3) * 0.5;
        }
      });
      break;
  }
}

// -------------------------------------------------------------------------
// Data flow particle animation
// -------------------------------------------------------------------------

function animateDataFlowParticles(group: THREE.Object3D, time: number): void {
  group.children.forEach((child) => {
    if (!(child instanceof THREE.InstancedMesh)) return;
    const curve = child.userData.curve as THREE.CatmullRomCurve3 | undefined;
    const count = child.userData.particleCount as number | undefined;
    if (!curve || !count) return;

    const dummy = new THREE.Object3D();
    for (let i = 0; i < count; i++) {
      const t = ((i / count) + time * 0.3) % 1;
      const p = curve.getPointAt(t);
      dummy.position.copy(p);
      dummy.updateMatrix();
      child.setMatrixAt(i, dummy.matrix);
    }
    child.instanceMatrix.needsUpdate = true;
  });
}

// -------------------------------------------------------------------------
// Transition helpers
// -------------------------------------------------------------------------

/** Smoothly interpolate a scalar value over time. */
export class AnimatedValue {
  private current: number;
  private target: number;
  private speed: number;

  constructor(initial: number, speed = 5) {
    this.current = initial;
    this.target = initial;
    this.speed = speed;
  }

  setTarget(value: number): void {
    this.target = value;
  }

  update(dt: number): number {
    const diff = this.target - this.current;
    this.current += diff * Math.min(this.speed * dt, 1);
    return this.current;
  }

  get value(): number {
    return this.current;
  }
}

/** Scale an object in/out on spawn/despawn. */
export function animateScale(
  object: THREE.Object3D,
  targetScale: number,
  dt: number,
  speed = 5,
): void {
  const s = object.scale.x;
  const diff = targetScale - s;
  const newS = s + diff * Math.min(speed * dt, 1);
  object.scale.setScalar(newS);
}
