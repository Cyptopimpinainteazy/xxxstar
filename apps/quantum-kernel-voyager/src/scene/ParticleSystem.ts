/**
 * @module scene/ParticleSystem
 * Object-pooled particle effects for thrusters, quantum tunneling,
 * block celebrations, and transaction flow visualizations.
 *
 * Uses THREE.Points with BufferGeometry for GPU-efficient rendering.
 * All pools are pre-allocated to avoid runtime GC pressure.
 */
import * as THREE from "three";

export type ParticleEffectKind = "thruster" | "quantum_tunnel" | "block_celebration" | "tx_flow";

interface Particle {
  position: THREE.Vector3;
  velocity: THREE.Vector3;
  life: number;
  maxLife: number;
  size: number;
  color: THREE.Color;
}

interface EffectPool {
  particles: Particle[];
  geometry: THREE.BufferGeometry;
  material: THREE.PointsMaterial;
  points: THREE.Points;
  activeCount: number;
  maxCount: number;
}

/** @internal base pool size for each particle kind */
export const DEFAULT_POOL_SIZE = 512;

const EFFECT_CONFIGS: Record<ParticleEffectKind, {
  maxCount: number;
  baseColor: number;
  baseSize: number;
  baseLife: number;
}> = {
  thruster: { maxCount: 256, baseColor: 0x00f0ff, baseSize: 0.15, baseLife: 0.5 },
  quantum_tunnel: { maxCount: 512, baseColor: 0xff00ff, baseSize: 0.2, baseLife: 1.5 },
  block_celebration: { maxCount: 128, baseColor: 0xffd700, baseSize: 0.3, baseLife: 2.0 },
  tx_flow: { maxCount: 256, baseColor: 0x00ff88, baseSize: 0.1, baseLife: 1.0 },
};

export class ParticleSystem {
  private readonly pools = new Map<ParticleEffectKind, EffectPool>();
  private readonly parentScene: THREE.Scene;

  constructor(scene: THREE.Scene) {
    this.parentScene = scene;
    for (const [kind, config] of Object.entries(EFFECT_CONFIGS)) {
      this.createPool(kind as ParticleEffectKind, config);
    }
  }

  /**
   * Emit particles of a given effect at a world position.
   * @param kind Effect type
   * @param origin World-space emission center
   * @param count Number of particles to emit (capped by pool)
   * @param direction Optional emission direction (default: random sphere)
   */
  emit(
    kind: ParticleEffectKind,
    origin: THREE.Vector3,
    count: number,
    direction?: THREE.Vector3,
  ): void {
    const pool = this.pools.get(kind);
    if (!pool) return;
    const config = EFFECT_CONFIGS[kind];

    for (let i = 0; i < count; i++) {
      // Find a dead particle to reuse
      const idx = this.findDeadParticle(pool);
      if (idx === -1) break;

      const p = pool.particles[idx];
      p.position.copy(origin);
      p.life = config.baseLife * (0.8 + Math.random() * 0.4);
      p.maxLife = p.life;
      p.size = config.baseSize * (0.7 + Math.random() * 0.6);
      p.color.set(config.baseColor);

      if (direction) {
        p.velocity.copy(direction).multiplyScalar(2 + Math.random() * 3);
        // Add spread
        p.velocity.x += (Math.random() - 0.5) * 1.5;
        p.velocity.y += (Math.random() - 0.5) * 1.5;
        p.velocity.z += (Math.random() - 0.5) * 1.5;
      } else {
        // Random sphere emission
        const theta = Math.random() * Math.PI * 2;
        const phi = Math.acos(Math.random() * 2 - 1);
        const speed = 1 + Math.random() * 4;
        p.velocity.set(
          Math.sin(phi) * Math.cos(theta) * speed,
          Math.sin(phi) * Math.sin(theta) * speed,
          Math.cos(phi) * speed,
        );
      }

      pool.activeCount = Math.max(pool.activeCount, idx + 1);
    }
  }

  /** Must be called every frame with delta time in seconds. */
  update(dt: number): void {
    for (const pool of this.pools.values()) {
      this.updatePool(pool, dt);
    }
  }

  /** Remove all particles and dispose GPU resources. */
  dispose(): void {
    for (const pool of this.pools.values()) {
      this.parentScene.remove(pool.points);
      pool.geometry.dispose();
      pool.material.dispose();
    }
    this.pools.clear();
  }

  // -----------------------------------------------------------------------
  // Private
  // -----------------------------------------------------------------------

  private createPool(kind: ParticleEffectKind, config: typeof EFFECT_CONFIGS[ParticleEffectKind]): void {
    const maxCount = config.maxCount;
    const particles: Particle[] = [];

    const positions = new Float32Array(maxCount * 3);
    const colors = new Float32Array(maxCount * 3);
    const sizes = new Float32Array(maxCount);

    for (let i = 0; i < maxCount; i++) {
      particles.push({
        position: new THREE.Vector3(0, -9999, 0),
        velocity: new THREE.Vector3(),
        life: 0,
        maxLife: 1,
        size: 0,
        color: new THREE.Color(config.baseColor),
      });
      positions[i * 3] = 0;
      positions[i * 3 + 1] = -9999;
      positions[i * 3 + 2] = 0;
      sizes[i] = 0;
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
    geometry.setAttribute("size", new THREE.BufferAttribute(sizes, 1));

    const material = new THREE.PointsMaterial({
      size: config.baseSize,
      vertexColors: true,
      transparent: true,
      opacity: 0.8,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      sizeAttenuation: true,
    });

    const points = new THREE.Points(geometry, material);
    points.frustumCulled = false;
    this.parentScene.add(points);

    this.pools.set(kind, {
      particles,
      geometry,
      material,
      points,
      activeCount: 0,
      maxCount,
    });
  }

  private updatePool(pool: EffectPool, dt: number): void {
    const posAttr = pool.geometry.getAttribute("position") as THREE.BufferAttribute;
    const colorAttr = pool.geometry.getAttribute("color") as THREE.BufferAttribute;
    const sizeAttr = pool.geometry.getAttribute("size") as THREE.BufferAttribute;
    let maxActive = 0;

    for (let i = 0; i < pool.activeCount; i++) {
      const p = pool.particles[i];
      if (p.life <= 0) continue;

      p.life -= dt;
      if (p.life <= 0) {
        // Kill particle — move offscreen
        posAttr.setXYZ(i, 0, -9999, 0);
        sizeAttr.setX(i, 0);
        continue;
      }

      // Physics
      p.velocity.y -= 0.5 * dt; // Slight gravity
      p.position.addScaledVector(p.velocity, dt);

      // Fade by life ratio
      const ratio = p.life / p.maxLife;

      posAttr.setXYZ(i, p.position.x, p.position.y, p.position.z);
      colorAttr.setXYZ(i, p.color.r * ratio, p.color.g * ratio, p.color.b * ratio);
      sizeAttr.setX(i, p.size * ratio);

      maxActive = i + 1;
    }

    posAttr.needsUpdate = true;
    colorAttr.needsUpdate = true;
    sizeAttr.needsUpdate = true;
    pool.geometry.setDrawRange(0, maxActive);
    pool.activeCount = maxActive;
  }

  private findDeadParticle(pool: EffectPool): number {
    // First pass: find dead particle in active range
    for (let i = 0; i < pool.activeCount; i++) {
      if (pool.particles[i].life <= 0) return i;
    }
    // Expand active count if space remains
    if (pool.activeCount < pool.maxCount) {
      return pool.activeCount;
    }
    return -1;
  }
}
