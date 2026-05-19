/**
 * Custom hook for cursor tracking logic.
 *
 * Converts screen-space cursor coordinates into normalized device coordinates
 * and calculates target gaze angles for the eyeball with smooth quaternion
 * interpolation (SLERP).
 *
 * @returns {{ screenX, screenY, gazeAngle, pupilDilation, isActive }}
 */
import { useCallback, useEffect, useRef, useState } from "react";
import * as THREE from "three";

/** Configuration for tracking sensitivity and interpolation */
export interface EyeballTrackingConfig {
  /** SLERP interpolation factor per frame (0–1). Lower = smoother. Default 0.08 */
  easing: number;
  /** Maximum pitch angle in radians. Default π/6 */
  maxPitch: number;
  /** Maximum yaw angle in radians. Default π/5 */
  maxYaw: number;
  /** Duration (ms) for returning to neutral gaze when cursor leaves. Default 800 */
  returnDuration: number;
  /** Pupil dilation range [min, max]. Default [0.3, 1.0] */
  dilationRange: [number, number];
}

const DEFAULT_CONFIG: EyeballTrackingConfig = {
  easing: 1.0,
  maxPitch: Math.PI / 4,
  maxYaw: Math.PI / 3,
  returnDuration: 50,
  dilationRange: [0.3, 1.0],
};

export interface EyeballTrackingState {
  /** Normalised X position (-1 to 1) */
  screenX: number;
  /** Normalised Y position (-1 to 1) */
  screenY: number;
  /** Current gaze direction as Euler { x(pitch), y(yaw) } in radians */
  gazeAngle: { x: number; y: number };
  /** Current pupil dilation factor (0–1) */
  pupilDilation: number;
  /** Whether cursor is inside the viewport */
  isActive: boolean;
}

/**
 * Pure math: compute target quaternion from normalised cursor position.
 *
 * @param ndcX - cursor X in NDC (-1..1)
 * @param ndcY - cursor Y in NDC (-1..1)
 * @param maxYaw - max yaw angle (radians)
 * @param maxPitch - max pitch angle (radians)
 */
export function computeTargetQuaternion(
  ndcX: number,
  ndcY: number,
  maxYaw: number,
  maxPitch: number,
): THREE.Quaternion {
  const yaw = ndcX * maxYaw;
  const pitch = -ndcY * maxPitch; // invert Y so looking up when cursor is up
  const euler = new THREE.Euler(pitch, yaw, 0, "YXZ");
  return new THREE.Quaternion().setFromEuler(euler);
}

/**
 * Pure math: compute pupil dilation from cursor distance to centre.
 *
 * Larger distance → pupil contracts slightly (simulates focus change).
 * Centre gaze → fully dilated.
 */
export function computeDilation(
  ndcX: number,
  ndcY: number,
  range: [number, number],
): number {
  const dist = Math.min(1, Math.sqrt(ndcX * ndcX + ndcY * ndcY));
  const [min, max] = range;
  return max - dist * (max - min);
}

export function useEyeballTracking(
  config: Partial<EyeballTrackingConfig> = {},
): EyeballTrackingState {
  const cfg = { ...DEFAULT_CONFIG, ...config };

  const [state, setState] = useState<EyeballTrackingState>({
    screenX: 0,
    screenY: 0,
    gazeAngle: { x: 0, y: 0 },
    pupilDilation: cfg.dilationRange[1],
    isActive: false,
  });

  const currentQuat = useRef(new THREE.Quaternion());
  const targetQuat = useRef(new THREE.Quaternion());
  const ndcRef = useRef({ x: 0, y: 0 });
  const activeRef = useRef(false);
  const rafRef = useRef<number>(0);

  // Track cursor position
  const onMouseMove = useCallback(
    (e: MouseEvent) => {
      const ndcX = (2 * e.clientX) / window.innerWidth - 1;
      const ndcY = 1 - (2 * e.clientY) / window.innerHeight;
      ndcRef.current = { x: ndcX, y: ndcY };
      activeRef.current = true;

      targetQuat.current = computeTargetQuaternion(
        ndcX,
        ndcY,
        cfg.maxYaw,
        cfg.maxPitch,
      );
    },
    [cfg.maxYaw, cfg.maxPitch],
  );

  const onMouseLeave = useCallback(() => {
    activeRef.current = false;
    // Target returns to identity (neutral forward gaze)
    targetQuat.current.identity();
    ndcRef.current = { x: 0, y: 0 };
  }, []);

  // Animation loop for smooth interpolation
  useEffect(() => {
    let running = true;

    const tick = () => {
      if (!running) return;

      // If easing is 1.0 or higher, skip slerp entirely for instant tracking
      if (cfg.easing >= 1.0) {
        currentQuat.current.copy(targetQuat.current);
      } else {
        const easeFactor = activeRef.current
          ? cfg.easing
          : cfg.easing * 0.5; // slower return to neutral
        currentQuat.current.slerp(targetQuat.current, easeFactor);
      }

      // Extract Euler from interpolated quaternion
      const euler = new THREE.Euler().setFromQuaternion(
        currentQuat.current,
        "YXZ",
      );

      const dilation = computeDilation(
        ndcRef.current.x,
        ndcRef.current.y,
        cfg.dilationRange,
      );

      setState({
        screenX: ndcRef.current.x,
        screenY: ndcRef.current.y,
        gazeAngle: { x: euler.x, y: euler.y },
        pupilDilation: dilation,
        isActive: activeRef.current,
      });

      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);

    return () => {
      running = false;
      cancelAnimationFrame(rafRef.current);
    };
  }, [cfg.easing, cfg.dilationRange]);

  // Attach global listeners
  useEffect(() => {
    document.addEventListener("mousemove", onMouseMove, { passive: true });
    document.addEventListener("mouseleave", onMouseLeave);

    return () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseleave", onMouseLeave);
    };
  }, [onMouseMove, onMouseLeave]);

  return state;
}
