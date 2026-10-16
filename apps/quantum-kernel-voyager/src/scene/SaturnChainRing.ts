/**
 * @module scene/SaturnChainRing
 * Saturn-style orbital ring made of interlocking gold chain links.
 *
 * Each link is a small torus rotated 90° alternately to simulate
 * real chain link interlocking. The entire ring tilts at ~25 degrees
 * and continuously rotates around the central globe.
 */
import * as THREE from "three";

export interface SaturnChainRingConfig {
  /** Distance from center to chain ring. Default = sun radius × 1.6. */
  ringRadius?: number;
  /** Number of individual chain links. Default = 56. */
  linkCount?: number;
  /** Torus tube radius of each link. Default = 0.35. */
  linkTubeRadius?: number;
  /** Torus radius of each link. Default = 0.8. */
  linkTorusRadius?: number;
  /** Tilt angle in degrees. Default = 25. */
  tiltDegrees?: number;
  /** 'left' tilts on +Z axis, 'right' on -Z axis. Default = 'right'. */
  tiltDirection?: "left" | "right";
  /** Seconds per full revolution. Default = 10. */
  revolutionPeriod?: number;
}

export class SaturnChainRing {
  readonly group: THREE.Group;

  private readonly revolutionSpeed: number; // radians per second
  private readonly linkMaterial: THREE.MeshStandardMaterial;

  constructor(config: SaturnChainRingConfig = {}) {
    const ringRadius = config.ringRadius ?? 19.2; // 12 * 1.6
    const linkCount = config.linkCount ?? 56;
    const linkTube = config.linkTubeRadius ?? 0.35;
    const linkTorus = config.linkTorusRadius ?? 0.8;
    const tiltDeg = config.tiltDegrees ?? 25;
    const tiltDir = config.tiltDirection ?? "right";
    const period = config.revolutionPeriod ?? 10;

    this.revolutionSpeed = (Math.PI * 2) / period;

    this.group = new THREE.Group();
    this.group.name = "saturn-chain-ring";

    // Apply tilt: rotate on Z axis so ring plane is angled
    const tiltRad = (tiltDeg * Math.PI) / 180;
    if (tiltDir === "right") {
      this.group.rotation.z = -tiltRad;
      this.group.rotation.x = tiltRad * 0.3; // slight X tilt for 3D perspective
    } else {
      this.group.rotation.z = tiltRad;
      this.group.rotation.x = -tiltRad * 0.3;
    }

    // Shared material for all links — polished gold PBR
    this.linkMaterial = new THREE.MeshStandardMaterial({
      color: 0xffd700,
      emissive: 0xb8860b,
      emissiveIntensity: 0.3,
      metalness: 0.95,
      roughness: 0.15,
    });

    // Shared geometry (reuse for all links)
    const linkGeo = new THREE.TorusGeometry(linkTorus, linkTube, 8, 16);

    // Place chain links along the ring circumference
    for (let i = 0; i < linkCount; i++) {
      const angle = (i / linkCount) * Math.PI * 2;
      const x = Math.cos(angle) * ringRadius;
      const z = Math.sin(angle) * ringRadius;

      const link = new THREE.Mesh(linkGeo, this.linkMaterial);
      link.position.set(x, 0, z);

      // Orient each link to face tangent direction along the ring
      link.lookAt(0, 0, 0);

      // Alternate 90° rotation to simulate interlocking chain
      if (i % 2 === 0) {
        link.rotateX(Math.PI / 2);
      }

      this.group.add(link);
    }
  }

  /** Call every frame with delta time to animate rotation. */
  update(dt: number): void {
    // Counterclockwise rotation around Y (the ring's local up axis)
    this.group.rotation.y -= this.revolutionSpeed * dt;
  }

  dispose(): void {
    // Geometry is shared — dispose once
    const firstChild = this.group.children[0] as THREE.Mesh | undefined;
    if (firstChild) {
      firstChild.geometry.dispose();
    }
    this.linkMaterial.dispose();
  }
}
