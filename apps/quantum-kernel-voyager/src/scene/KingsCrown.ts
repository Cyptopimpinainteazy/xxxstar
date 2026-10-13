/**
 * @module scene/KingsCrown
 * Wireframe king's crown perched atop the central sun globe.
 *
 * 5-pointed crown with fleur-de-lis-style peaks, constructed from
 * gold TubeGeometry along a CatmullRomCurve3. Tilted opposite to
 * the chain ring for visual balance. Gently bobs up and down.
 */
import * as THREE from "three";

export interface KingsCrownConfig {
  /** Base radius of the crown circle. Default = sun radius × 0.55. */
  baseRadius?: number;
  /** Height of the crown peaks above the base. Default = 5. */
  peakHeight?: number;
  /** Height of the valleys between peaks. Default = 1.5. */
  valleyHeight?: number;
  /** Number of crown points. Default = 5. */
  numPoints?: number;
  /** Tube thickness for the crown wireframe. Default = 0.2. */
  tubeRadius?: number;
  /** Y position above globe center. Default = sun radius + 1. */
  yOffset?: number;
  /** Tilt angle in degrees. Default = 18. */
  tiltDegrees?: number;
  /** 'left' or 'right' — should be opposite of ring tilt. Default = 'left'. */
  tiltDirection?: "left" | "right";
  /** Bob amplitude in units. Default = 0.3. */
  bobAmplitude?: number;
  /** Bob speed multiplier. Default = 1.5. */
  bobSpeed?: number;
}

export class KingsCrown {
  readonly group: THREE.Group;

  private readonly baseY: number;
  private readonly bobAmplitude: number;
  private readonly bobSpeed: number;
  private elapsed = 0;

  constructor(config: KingsCrownConfig = {}) {
    const baseRadius = config.baseRadius ?? 6.6; // 12 * 0.55
    const peakHeight = config.peakHeight ?? 5;
    const valleyHeight = config.valleyHeight ?? 1.5;
    const numPoints = config.numPoints ?? 5;
    const tubeRadius = config.tubeRadius ?? 0.2;
    const yOffset = config.yOffset ?? 13; // sits just above globe surface
    const tiltDeg = config.tiltDegrees ?? 18;
    const tiltDir = config.tiltDirection ?? "left";
    this.bobAmplitude = config.bobAmplitude ?? 0.3;
    this.bobSpeed = config.bobSpeed ?? 1.5;
    this.baseY = yOffset;

    this.group = new THREE.Group();
    this.group.name = "kings-crown";
    this.group.position.y = yOffset;

    // Apply tilt opposite to ring
    const tiltRad = (tiltDeg * Math.PI) / 180;
    if (tiltDir === "left") {
      this.group.rotation.z = tiltRad;
    } else {
      this.group.rotation.z = -tiltRad;
    }

    // -- Generate crown path points --
    // Alternating peaks and valleys around a circle, with small
    // fleur-de-lis accent bumps near each peak.
    const pathPoints: THREE.Vector3[] = [];
    const totalVertices = numPoints * 2; // peak + valley per point

    for (let i = 0; i <= totalVertices; i++) {
      const t = (i / totalVertices) * Math.PI * 2;
      const isPeak = i % 2 === 0;
      const height = isPeak ? peakHeight : valleyHeight;

      const x = Math.cos(t) * baseRadius;
      const z = Math.sin(t) * baseRadius;
      const y = height;

      pathPoints.push(new THREE.Vector3(x, y, z));

      // Add slight fleur-de-lis accent midway to peaks
      if (isPeak && i < totalVertices) {
        const tMid = ((i + 0.5) / totalVertices) * Math.PI * 2;
        const midHeight = peakHeight * 0.75;
        const innerRadius = baseRadius * 0.88;
        pathPoints.push(
          new THREE.Vector3(
            Math.cos(tMid) * innerRadius,
            midHeight,
            Math.sin(tMid) * innerRadius,
          ),
        );
      }
    }

    // Close the curve
    pathPoints.push(pathPoints[0].clone());

    const curve = new THREE.CatmullRomCurve3(pathPoints, true, "centripetal", 0.5);
    const tubeGeo = new THREE.TubeGeometry(curve, 128, tubeRadius, 8, true);

    // Gold emissive wireframe-style material
    const mat = new THREE.MeshStandardMaterial({
      color: 0xffd700,
      emissive: 0xffd700,
      emissiveIntensity: 0.6,
      metalness: 0.9,
      roughness: 0.15,
    });

    const crownMesh = new THREE.Mesh(tubeGeo, mat);
    crownMesh.name = "crown-tube";
    this.group.add(crownMesh);

    // Add wireframe overlay for cyberpunk edge lines
    const wireGeo = new THREE.EdgesGeometry(tubeGeo);
    const wireMat = new THREE.LineBasicMaterial({
      color: 0xffd700,
      transparent: true,
      opacity: 0.4,
    });
    const wireframe = new THREE.LineSegments(wireGeo, wireMat);
    wireframe.name = "crown-wireframe";
    this.group.add(wireframe);

    // Base ring (bottom circle of the crown)
    const ringCurvePoints: THREE.Vector3[] = [];
    const ringSegs = 64;
    for (let i = 0; i <= ringSegs; i++) {
      const a = (i / ringSegs) * Math.PI * 2;
      ringCurvePoints.push(
        new THREE.Vector3(
          Math.cos(a) * baseRadius,
          0,
          Math.sin(a) * baseRadius,
        ),
      );
    }
    const ringCurve = new THREE.CatmullRomCurve3(ringCurvePoints, true);
    const ringTubeGeo = new THREE.TubeGeometry(ringCurve, 64, tubeRadius * 1.2, 8, true);
    const ringMesh = new THREE.Mesh(ringTubeGeo, mat);
    ringMesh.name = "crown-base-ring";
    this.group.add(ringMesh);
  }

  /** Call every frame with delta time for bobbing animation. */
  update(dt: number): void {
    this.elapsed += dt;
    const bob = Math.sin(this.elapsed * this.bobSpeed) * this.bobAmplitude;
    this.group.position.y = this.baseY + bob;
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
