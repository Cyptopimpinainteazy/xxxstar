/**
 * @module scene/GoldRays
 * Radiating gold light rays emanating from the central sun.
 *
 * Uses icosahedron vertices for even spherical distribution.
 * Each ray is a billboard plane with additive blending and
 * animated pulsing opacity (sine wave with phase offsets).
 * Includes a central gold point light for volumetric feel.
 */
import * as THREE from "three";

export interface GoldRaysConfig {
  /** Number of major rays. Default = 14. */
  rayCount?: number;
  /** Min ray length beyond globe surface. Default = 8. */
  minLength?: number;
  /** Max ray length beyond globe surface. Default = 20. */
  maxLength?: number;
  /** Ray width. Default = 1.2. */
  rayWidth?: number;
  /** Distance from center where rays begin. Default = sun radius × 1.1. */
  startDistance?: number;
  /** Base opacity (0-1). Default = 0.45. */
  baseOpacity?: number;
  /** Pulse cycle duration in seconds. Default = 3.5. */
  cycleDuration?: number;
  /** Central point light intensity. Default = 5. */
  lightIntensity?: number;
  /** Central point light reach. Default = 100. */
  lightDistance?: number;
  /** Deterministic seed. Default = 42. */
  seed?: number;
}

interface RayData {
  direction: THREE.Vector3;
  height: number;
  phaseOffset: number;
  mesh: THREE.Mesh;
}

export class GoldRays {
  readonly group: THREE.Group;
  readonly light: THREE.PointLight;

  private readonly rays: RayData[] = [];
  private readonly baseOpacity: number;
  private readonly cycleDuration: number;
  private elapsed = 0;

  constructor(config: GoldRaysConfig = {}) {
    const rayCount = config.rayCount ?? 14;
    const minLen = config.minLength ?? 8;
    const maxLen = config.maxLength ?? 20;
    const rayWidth = config.rayWidth ?? 1.2;
    const startDist = config.startDistance ?? 13.2; // 12 * 1.1
    this.baseOpacity = config.baseOpacity ?? 0.45;
    this.cycleDuration = config.cycleDuration ?? 3.5;
    const lightIntensity = config.lightIntensity ?? 5;
    const lightDist = config.lightDistance ?? 100;
    const seed = config.seed ?? 42;

    this.group = new THREE.Group();
    this.group.name = "gold-rays";

    // Seeded pseudo-random
    let rngState = Math.sin(seed) * 10000;
    const seededRandom = (): number => {
      rngState = Math.sin(rngState) * 10000;
      return rngState - Math.floor(rngState);
    };

    // Generate icosahedron vertices for even spherical distribution
    const icoGeo = new THREE.IcosahedronGeometry(1, 0);
    const posAttr = icoGeo.attributes.position;
    const uniqueVerts = new Map<string, THREE.Vector3>();

    // Deduplicate icosahedron vertices
    for (let i = 0; i < posAttr.count; i++) {
      const v = new THREE.Vector3(
        posAttr.getX(i),
        posAttr.getY(i),
        posAttr.getZ(i),
      ).normalize();
      const key = `${v.x.toFixed(4)},${v.y.toFixed(4)},${v.z.toFixed(4)}`;
      if (!uniqueVerts.has(key)) {
        uniqueVerts.set(key, v);
      }
    }
    icoGeo.dispose();

    let directions = Array.from(uniqueVerts.values());

    // If we need more rays, add some random directions
    while (directions.length < rayCount) {
      const phi = Math.acos(1 - 2 * seededRandom());
      const theta = 2 * Math.PI * seededRandom();
      directions.push(
        new THREE.Vector3(
          Math.sin(phi) * Math.cos(theta),
          Math.sin(phi) * Math.sin(theta),
          Math.cos(phi),
        ).normalize(),
      );
    }
    // Trim to rayCount
    directions = directions.slice(0, rayCount);

    // Create ray billboards
    const rayGeo = new THREE.PlaneGeometry(1, 1); // scaled per-ray

    for (const dir of directions) {
      const height = minLen + seededRandom() * (maxLen - minLen);
      const phaseOffset = seededRandom() * Math.PI * 2;

      const mat = new THREE.MeshBasicMaterial({
        color: 0xffd700,
        transparent: true,
        opacity: this.baseOpacity,
        blending: THREE.AdditiveBlending,
        side: THREE.DoubleSide,
        depthWrite: false,
      });

      const mesh = new THREE.Mesh(rayGeo, mat);
      mesh.scale.set(rayWidth, height, 1);

      // Position along direction, offset from globe surface
      const pos = dir.clone().multiplyScalar(startDist + height * 0.5);
      mesh.position.copy(pos);

      // Orient to point away from center
      mesh.lookAt(0, 0, 0);
      mesh.rotateX(Math.PI / 2); // Align plane height along radial direction

      this.group.add(mesh);
      this.rays.push({ direction: dir, height, phaseOffset, mesh });
    }

    // Central point light for sun illumination
    this.light = new THREE.PointLight(0xffd700, lightIntensity, lightDist);
    this.light.name = "sun-point-light";
    this.group.add(this.light);
  }

  /** Call every frame with delta time for opacity pulsing. */
  update(dt: number): void {
    this.elapsed += dt;
    const omega = (2 * Math.PI) / this.cycleDuration;

    for (const ray of this.rays) {
      const opacity =
        this.baseOpacity +
        Math.sin(this.elapsed * omega + ray.phaseOffset) * 0.15;
      (ray.mesh.material as THREE.MeshBasicMaterial).opacity = Math.max(
        0.1,
        Math.min(1, opacity),
      );
    }
  }

  dispose(): void {
    // Geometry is shared, dispose once
    if (this.rays.length > 0) {
      this.rays[0].mesh.geometry.dispose();
    }
    for (const ray of this.rays) {
      (ray.mesh.material as THREE.Material).dispose();
    }
    this.light.dispose();
  }
}
