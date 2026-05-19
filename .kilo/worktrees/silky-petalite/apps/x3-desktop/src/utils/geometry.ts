/**
 * Geometry utilities — vector math, quaternion operations, collision checks.
 */
import * as THREE from "three";

/**
 * Clamp a value between min and max.
 */
export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/**
 * Linear interpolation between two values.
 */
export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

/**
 * Convert screen coordinates to normalised device coordinates.
 */
export function screenToNDC(
  clientX: number,
  clientY: number,
  width: number,
  height: number,
): { x: number; y: number } {
  return {
    x: (2 * clientX) / width - 1,
    y: 1 - (2 * clientY) / height,
  };
}

/**
 * Compute the target quaternion for eyeball gaze from NDC cursor position.
 */
export function gazeQuaternion(
  ndcX: number,
  ndcY: number,
  maxYaw: number,
  maxPitch: number,
): THREE.Quaternion {
  const yaw = ndcX * maxYaw;
  const pitch = -ndcY * maxPitch;
  const euler = new THREE.Euler(pitch, yaw, 0, "YXZ");
  return new THREE.Quaternion().setFromEuler(euler);
}

/**
 * Check if a rectangle is fully within viewport bounds.
 */
export function isWithinViewport(
  x: number,
  y: number,
  w: number,
  h: number,
  viewW: number,
  viewH: number,
): boolean {
  return x >= 0 && y >= 0 && x + w <= viewW && y + h <= viewH;
}

/**
 * Constrain a window position so at least 50px of the title bar is visible.
 */
export function constrainPosition(
  x: number,
  y: number,
  w: number,
  _h: number,
  viewW: number,
  viewH: number,
): { x: number; y: number } {
  return {
    x: clamp(x, -(w - 50), viewW - 50),
    y: clamp(y, 0, viewH - 40),
  };
}
