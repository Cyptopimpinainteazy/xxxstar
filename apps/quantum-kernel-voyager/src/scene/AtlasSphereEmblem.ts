/**
 * @module scene/AtlasSphereEmblem
 * Orchestrates the full Atlas Sphere sun emblem:
 *   1. CentralSunGlobe — molten gold wireframe sphere
 *   2. SaturnChainRing — interlocking gold chain orbit
 *   3. KingsCrown      — wireframe crown tilted opposite to ring
 *   4. GoldRays         — radiating corona light rays
 *
 * Designed to replace the simple sun in SolarSystemBuilder.
 * Manages lifecycle, per-frame updates, and disposal of all sub-elements.
 */
import * as THREE from "three";
import { CentralSunGlobe } from "./CentralSunGlobe";
import { SaturnChainRing } from "./SaturnChainRing";
import { KingsCrown } from "./KingsCrown";
import { GoldRays } from "./GoldRays";

export interface AtlasSphereEmblemConfig {
  /** Sun visual radius. Default = 12 (matches SUN_RADIUS). */
  sunRadius?: number;
  /** Ring tilt side: 'left' or 'right'. Default = 'right'. */
  ringTiltSide?: "left" | "right";
  /** Enable the kings crown. Default = true. */
  showCrown?: boolean;
  /** Enable the gold corona rays. Default = true. */
  showRays?: boolean;
  /** Enable slow auto-rotation of entire emblem. Default = false. */
  autoRotate?: boolean;
  /** Auto-rotation speed in rad/s. Default = 0.05 (~2 min / revolution). */
  autoRotateSpeed?: number;
}

export class AtlasSphereEmblem {
  readonly group: THREE.Group;

  /** The central point light — exposed so SolarSystemBuilder can read intensity. */
  readonly sunLight: THREE.PointLight;

  private readonly globe: CentralSunGlobe;
  private readonly ring: SaturnChainRing;
  private readonly crown: KingsCrown | null;
  private readonly rays: GoldRays | null;
  private readonly autoRotate: boolean;
  private readonly autoRotateSpeed: number;

  constructor(config: AtlasSphereEmblemConfig = {}) {
    const sunRadius = config.sunRadius ?? 12;
    const ringTilt = config.ringTiltSide ?? "right";
    const crownTilt = ringTilt === "right" ? "left" : "right";
    const showCrown = config.showCrown ?? true;
    const showRays = config.showRays ?? true;
    this.autoRotate = config.autoRotate ?? false;
    this.autoRotateSpeed = config.autoRotateSpeed ?? 0.05;

    this.group = new THREE.Group();
    this.group.name = "atlas-sphere-emblem";

    // 1. Central Globe
    this.globe = new CentralSunGlobe({
      radius: sunRadius,
      segments: 64,
      wireOpacity: 0.95,
      showLogo: true,
      logoText: "X3",
    });
    this.group.add(this.globe.group);

    // 2. Saturn Chain Ring
    this.ring = new SaturnChainRing({
      ringRadius: sunRadius * 1.6,
      linkCount: 56,
      linkTorusRadius: sunRadius * 0.065,
      linkTubeRadius: sunRadius * 0.028,
      tiltDegrees: 25,
      tiltDirection: ringTilt,
      revolutionPeriod: 10,
    });
    this.group.add(this.ring.group);

    // 3. Kings Crown (opposite tilt)
    if (showCrown) {
      this.crown = new KingsCrown({
        baseRadius: sunRadius * 0.55,
        peakHeight: sunRadius * 0.42,
        valleyHeight: sunRadius * 0.12,
        numPoints: 5,
        tubeRadius: sunRadius * 0.017,
        yOffset: sunRadius * 1.08,
        tiltDegrees: 18,
        tiltDirection: crownTilt,
        bobAmplitude: 0.3,
        bobSpeed: 1.5,
      });
      this.group.add(this.crown.group);
    } else {
      this.crown = null;
    }

    // 4. Gold Rays
    if (showRays) {
      this.rays = new GoldRays({
        rayCount: 14,
        minLength: sunRadius * 0.67,
        maxLength: sunRadius * 1.67,
        rayWidth: sunRadius * 0.1,
        startDistance: sunRadius * 1.1,
        baseOpacity: 0.45,
        cycleDuration: 3.5,
        lightIntensity: 5,
        lightDistance: sunRadius * 8,
      });
      this.group.add(this.rays.group);
      this.sunLight = this.rays.light;
    } else {
      // Still need a light for the sun even without ray sprites
      this.sunLight = new THREE.PointLight(0xffd700, 5, sunRadius * 8);
      this.sunLight.name = "sun-point-light";
      this.group.add(this.sunLight);
      this.rays = null;
    }
  }

  /**
   * Per-frame update. Call from the game loop with delta time.
   * Also accepts optional sunMetrics for pulsing intensity.
   */
  update(dt: number, healthScore?: number): void {
    this.globe.update(dt);
    this.ring.update(dt);
    this.crown?.update(dt);
    this.rays?.update(dt);

    if (this.autoRotate) {
      this.group.rotation.y += this.autoRotateSpeed * dt;
    }

    // Modulate sun light intensity from health score (if provided)
    if (healthScore !== undefined) {
      this.sunLight.intensity = 3 + healthScore * 4;
    }
  }

  /**
   * Apply a uniform scale pulse (called from SolarSystemBuilder.update
   * to match the existing block-time pulse behavior).
   */
  setPulseScale(scale: number): void {
    this.globe.group.scale.setScalar(scale);
  }

  dispose(): void {
    this.globe.dispose();
    this.ring.dispose();
    this.crown?.dispose();
    this.rays?.dispose();
    if (!this.rays) {
      this.sunLight.dispose();
    }
  }
}
