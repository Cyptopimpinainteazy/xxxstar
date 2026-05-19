/**
 * Unit tests for eyeball tracking mathematics.
 *
 * Tests cursor → NDC conversion, quaternion computation, pupil dilation,
 * and SLERP interpolation accuracy.
 *
 * Invariants tested: INV-UI-001 (eyeball tracks cursor within max angles)
 */
import { describe, it, expect } from "vitest";
import * as THREE from "three";
import {
  computeTargetQuaternion,
  computeDilation,
} from "../../src/components/eyeball/useEyeballTracking";
import { screenToNDC, gazeQuaternion, clamp, lerp } from "../../src/utils/geometry";

describe("screenToNDC", () => {
  it("returns (0, 0) for viewport center", () => {
    const { x, y } = screenToNDC(500, 500, 1000, 1000);
    expect(x).toBe(0);
    expect(y).toBe(0);
  });

  it("returns (-1, 1) for top-left corner", () => {
    const { x, y } = screenToNDC(0, 0, 1000, 1000);
    expect(x).toBe(-1);
    expect(y).toBe(1);
  });

  it("returns (1, -1) for bottom-right corner", () => {
    const { x, y } = screenToNDC(1000, 1000, 1000, 1000);
    expect(x).toBe(1);
    expect(y).toBe(-1);
  });

  it("handles non-square viewports correctly", () => {
    const { x, y } = screenToNDC(1920, 540, 1920, 1080);
    expect(x).toBe(1);
    expect(y).toBe(0);
  });
});

describe("computeTargetQuaternion", () => {
  const maxYaw = Math.PI / 5;
  const maxPitch = Math.PI / 6;

  it("returns identity quaternion for (0, 0) cursor position", () => {
    const q = computeTargetQuaternion(0, 0, maxYaw, maxPitch);
    const identity = new THREE.Quaternion();
    expect(q.angleTo(identity)).toBeCloseTo(0, 5);
  });

  it("produces yaw-only rotation for horizontal cursor movement", () => {
    const q = computeTargetQuaternion(1, 0, maxYaw, maxPitch);
    const euler = new THREE.Euler().setFromQuaternion(q, "YXZ");
    expect(euler.y).toBeCloseTo(maxYaw, 4);
    expect(euler.x).toBeCloseTo(0, 4);
  });

  it("produces pitch-only rotation for vertical cursor movement", () => {
    const q = computeTargetQuaternion(0, 1, maxYaw, maxPitch);
    const euler = new THREE.Euler().setFromQuaternion(q, "YXZ");
    expect(euler.y).toBeCloseTo(0, 4);
    expect(Math.abs(euler.x)).toBeCloseTo(maxPitch, 4);
  });

  it("quaternion angle never exceeds max bounds", () => {
    // Check many positions
    for (let x = -1; x <= 1; x += 0.25) {
      for (let y = -1; y <= 1; y += 0.25) {
        const q = computeTargetQuaternion(x, y, maxYaw, maxPitch);
        const euler = new THREE.Euler().setFromQuaternion(q, "YXZ");
        expect(Math.abs(euler.y)).toBeLessThanOrEqual(maxYaw + 0.001);
        expect(Math.abs(euler.x)).toBeLessThanOrEqual(maxPitch + 0.001);
      }
    }
  });
});

describe("gazeQuaternion (geometry util)", () => {
  it("matches computeTargetQuaternion output", () => {
    const maxYaw = Math.PI / 5;
    const maxPitch = Math.PI / 6;
    const q1 = computeTargetQuaternion(0.5, -0.3, maxYaw, maxPitch);
    const q2 = gazeQuaternion(0.5, -0.3, maxYaw, maxPitch);
    expect(q1.angleTo(q2)).toBeCloseTo(0, 5);
  });
});

describe("computeDilation", () => {
  it("returns max dilation at centre", () => {
    const d = computeDilation(0, 0, [0.3, 1.0]);
    expect(d).toBe(1.0);
  });

  it("returns min dilation at edge (1,0)", () => {
    const d = computeDilation(1, 0, [0.3, 1.0]);
    expect(d).toBeCloseTo(0.3, 4);
  });

  it("clamps distance to 1.0 for far positions", () => {
    const d = computeDilation(2, 2, [0.3, 1.0]);
    expect(d).toBeCloseTo(0.3, 4);
  });

  it("interpolates smoothly between min and max", () => {
    const d = computeDilation(0.5, 0, [0.3, 1.0]);
    expect(d).toBeGreaterThan(0.3);
    expect(d).toBeLessThan(1.0);
    // At distance 0.5: dilation = 1.0 - 0.5 * 0.7 = 0.65
    expect(d).toBeCloseTo(0.65, 4);
  });
});

describe("SLERP interpolation", () => {
  it("slerp with t=0 returns start quaternion", () => {
    const q1 = new THREE.Quaternion().setFromEuler(
      new THREE.Euler(0.1, 0.2, 0),
    );
    const q2 = new THREE.Quaternion().setFromEuler(
      new THREE.Euler(0.5, 0.8, 0),
    );
    const result = q1.clone().slerp(q2, 0);
    expect(q1.angleTo(result)).toBeCloseTo(0, 5);
  });

  it("slerp with t=1 returns end quaternion", () => {
    const q1 = new THREE.Quaternion().setFromEuler(
      new THREE.Euler(0.1, 0.2, 0),
    );
    const q2 = new THREE.Quaternion().setFromEuler(
      new THREE.Euler(0.5, 0.8, 0),
    );
    const result = q1.clone().slerp(q2, 1);
    expect(q2.angleTo(result)).toBeCloseTo(0, 5);
  });

  it("slerp produces intermediate rotation", () => {
    const q1 = new THREE.Quaternion();
    const q2 = new THREE.Quaternion().setFromEuler(
      new THREE.Euler(0, Math.PI / 4, 0),
    );
    const result = q1.clone().slerp(q2, 0.5);
    const euler = new THREE.Euler().setFromQuaternion(result, "YXZ");
    expect(euler.y).toBeCloseTo(Math.PI / 8, 3);
  });
});

describe("utility functions", () => {
  it("clamp constrains values", () => {
    expect(clamp(5, 0, 10)).toBe(5);
    expect(clamp(-1, 0, 10)).toBe(0);
    expect(clamp(15, 0, 10)).toBe(10);
  });

  it("lerp interpolates correctly", () => {
    expect(lerp(0, 10, 0)).toBe(0);
    expect(lerp(0, 10, 1)).toBe(10);
    expect(lerp(0, 10, 0.5)).toBe(5);
  });
});
