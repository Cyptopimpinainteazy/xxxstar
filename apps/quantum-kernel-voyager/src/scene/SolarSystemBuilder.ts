/**
 * @module scene/SolarSystemBuilder
 * Constructs the 3D solar system: sun, planets, orbit rings, starfield.
 *
 * Reads from PlanetRegistry + OrbitCalculator to place everything.
 * Manages the lifecycle of all solar system entities in the SceneManager.
 * Call update() every frame to advance orbits and sync positions.
 */
import * as THREE from "three";
import type { SceneEntity } from "../types/scene";
import type { PlanetEntity, SunMetrics } from "../types/solar-system";
import type { SceneManager } from "./SceneManager";
import type { PlanetRegistry } from "../services/PlanetRegistry";
import { advanceOrbits, getPlanetPosition } from "../services/OrbitCalculator";
import { SUN_RADIUS } from "../config/solar-system-config";
import { AtlasSphereEmblem } from "./AtlasSphereEmblem";

const SUN_ENTITY_ID = "solar-sun";
const STARFIELD_NAME = "solar-starfield";
const ORBIT_RING_PREFIX = "orbit-ring-";
const PLANET_PREFIX = "solar-planet-";

export class SolarSystemBuilder {
  private readonly sm: SceneManager;
  private readonly registry: PlanetRegistry;
  private sunEmblem: AtlasSphereEmblem | null = null;
  private builtPlanetIds = new Set<string>();
  private timeScale = 1;

  constructor(sceneManager: SceneManager, registry: PlanetRegistry) {
    this.sm = sceneManager;
    this.registry = registry;
  }

  /** Build the entire solar system scene. */
  build(): void {
    this.buildStarfield();
    this.buildSun();
    this.buildAllPlanets();
  }

  /** Remove all solar system entities from the scene. */
  teardown(): void {
    // Remove sun
    this.sm.removeEntity(SUN_ENTITY_ID);
    this.sunEmblem?.dispose();
    this.sunEmblem = null;

    // Remove starfield
    const stars = this.sm.scene.getObjectByName(STARFIELD_NAME);
    if (stars) this.sm.scene.remove(stars);

    // Remove all planets + orbit rings
    for (const pid of this.builtPlanetIds) {
      this.sm.removeEntity(PLANET_PREFIX + pid);
      const ring = this.sm.scene.getObjectByName(ORBIT_RING_PREFIX + pid);
      if (ring) this.sm.scene.remove(ring);
    }
    this.builtPlanetIds.clear();
  }

  setTimeScale(s: number): void {
    this.timeScale = s;
  }

  // -----------------------------------------------------------------------
  // Frame update — advance orbits, sync positions, animate sun
  // -----------------------------------------------------------------------

  update(dt: number, sunMetrics: SunMetrics): void {
    const planets = this.registry.getAll();

    // Advance orbits
    advanceOrbits(planets, dt, this.timeScale);

    // Sync 3D positions
    for (const p of planets) {
      const entityId = PLANET_PREFIX + p.id;
      const obj = this.sm.getObject(entityId);
      if (!obj) {
        // Planet was added dynamically — build it
        this.buildPlanet(p);
        continue;
      }
      const pos = getPlanetPosition(p);
      obj.position.set(pos.x, pos.y, pos.z);
      // Axial rotation
      obj.rotation.y += dt * 0.15;
    }

    // Animate sun — delegate to AtlasSphereEmblem
    if (this.sunEmblem) {
      this.sunEmblem.update(dt, sunMetrics.healthScore);

      // Block-time pulse (matches original behavior)
      const pulseRate = 1.0 / Math.max(sunMetrics.blockTime, 1);
      const time = performance.now() * 0.001;
      const pulse = 1 + Math.sin(time * pulseRate * Math.PI * 2) * 0.03;
      this.sunEmblem.setPulseScale(pulse);
    }
  }

  // -----------------------------------------------------------------------
  // Build helpers
  // -----------------------------------------------------------------------

  private buildStarfield(): void {
    const count = 3000;
    const geo = new THREE.BufferGeometry();
    const positions = new Float32Array(count * 3);
    const colors = new Float32Array(count * 3);

    for (let i = 0; i < count; i++) {
      const i3 = i * 3;
      const r = 400 + Math.random() * 1200;
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      positions[i3] = r * Math.sin(phi) * Math.cos(theta);
      positions[i3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      positions[i3 + 2] = r * Math.cos(phi);

      const tint = Math.random();
      if (tint < 0.25) {
        colors[i3] = 0; colors[i3 + 1] = 0.94; colors[i3 + 2] = 1; // cyan
      } else if (tint < 0.4) {
        colors[i3] = 1; colors[i3 + 1] = 0; colors[i3 + 2] = 1; // magenta
      } else if (tint < 0.5) {
        colors[i3] = 1; colors[i3 + 1] = 0.84; colors[i3 + 2] = 0; // gold
      } else {
        colors[i3] = 0.85; colors[i3 + 1] = 0.85; colors[i3 + 2] = 1; // cool white
      }
    }

    geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geo.setAttribute("color", new THREE.BufferAttribute(colors, 3));

    const mat = new THREE.PointsMaterial({
      size: 1.0,
      vertexColors: true,
      sizeAttenuation: true,
      transparent: true,
      opacity: 0.85,
    });
    const stars = new THREE.Points(geo, mat);
    stars.name = STARFIELD_NAME;
    this.sm.scene.add(stars);
  }

  private buildSun(): void {
    // Full Atlas Sphere emblem: golden wireframe sun + chain ring +
    // king's crown + radiating gold rays
    const emblem = new AtlasSphereEmblem({
      sunRadius: SUN_RADIUS,
      ringTiltSide: "right",
      showCrown: false,
      showRays: false,
      autoRotate: false, // rotation handled by update()
    });

    this.sunEmblem = emblem;

    // Register as scene entity
    const entity: SceneEntity = {
      id: SUN_ENTITY_ID,
      kind: "planet",
      position: { x: 0, y: 0, z: 0 },
      label: "Atlas Sphere L1",
      metadata: { isSun: true },
    };
    this.sm.addEntity(entity, emblem.group);
  }

  private buildAllPlanets(): void {
    for (const p of this.registry.getAll()) {
      this.buildPlanet(p);
    }
  }

  private buildPlanet(planet: PlanetEntity): void {
    if (this.builtPlanetIds.has(planet.id)) return;

    const entityId = PLANET_PREFIX + planet.id;
    const pos = getPlanetPosition(planet);

    // Planet sphere
    const group = new THREE.Group();
    const radius = Math.max(planet.planetRadius, 1);
    const geo = new THREE.SphereGeometry(radius, 32, 32);

    // Surface material varies by surface type
    const mat = this.createPlanetMaterial(planet);
    const mesh = new THREE.Mesh(geo, mat);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    group.add(mesh);

    // Atmosphere (if applicable)
    if (planet.hasAtmosphere) {
      const atmosGeo = new THREE.SphereGeometry(radius * 1.08, 24, 24);
      const atmosColor = planet.color;
      const atmosMat = new THREE.MeshBasicMaterial({
        color: atmosColor,
        transparent: true,
        opacity: 0.12,
        side: THREE.BackSide,
      });
      group.add(new THREE.Mesh(atmosGeo, atmosMat));
    }

    // Exchange planets get rings (Saturn-like)
    if (planet.type === "exchange") {
      const ringGeo = new THREE.RingGeometry(radius * 1.4, radius * 2.0, 64);
      const ringMat = new THREE.MeshBasicMaterial({
        color: planet.color,
        transparent: true,
        opacity: 0.25,
        side: THREE.DoubleSide,
      });
      const ring = new THREE.Mesh(ringGeo, ringMat);
      ring.rotation.x = Math.PI * 0.4;
      group.add(ring);
    }

    // Bridge planets get connection glow
    if (planet.type === "bridge") {
      const beamGeo = new THREE.CylinderGeometry(0.1, 0.1, radius * 3, 8);
      const beamMat = new THREE.MeshBasicMaterial({
        color: 0x00f0ff,
        transparent: true,
        opacity: 0.3,
      });
      const beam = new THREE.Mesh(beamGeo, beamMat);
      beam.rotation.z = Math.PI / 2;
      beam.position.y = radius * 2;
      group.add(beam);
    }

    // Name label (simple point light as visual cue)
    const labelLight = new THREE.PointLight(planet.color, 0.5, radius * 4);
    labelLight.position.y = radius * 1.3;
    group.add(labelLight);

    group.position.set(pos.x, pos.y, pos.z);

    const entity: SceneEntity = {
      id: entityId,
      kind: "planet",
      position: pos,
      label: planet.name,
      metadata: { planetId: planet.id, planetType: planet.type },
    };

    this.sm.addEntity(entity, group);
    this.builtPlanetIds.add(planet.id);

    // Orbit ring
    this.buildOrbitRing(planet);
  }

  private buildOrbitRing(planet: PlanetEntity): void {
    const ringName = ORBIT_RING_PREFIX + planet.id;
    // Remove old ring if exists
    const old = this.sm.scene.getObjectByName(ringName);
    if (old) this.sm.scene.remove(old);

    const curve = new THREE.EllipseCurve(0, 0, planet.orbitRadius, planet.orbitRadius, 0, Math.PI * 2, false, 0);
    const points = curve.getPoints(128);
    const geo = new THREE.BufferGeometry().setFromPoints(
      points.map((p) => new THREE.Vector3(p.x, 0, p.y)),
    );
    const mat = new THREE.LineBasicMaterial({
      color: planet.color,
      transparent: true,
      opacity: 0.08,
    });
    const line = new THREE.LineLoop(geo, mat);
    // Apply orbit inclination
    line.rotation.x = planet.orbitInclination * 0.3;
    line.name = ringName;
    this.sm.scene.add(line);
  }

  private createPlanetMaterial(planet: PlanetEntity): THREE.Material {
    switch (planet.surfaceType) {
      case "tech":
        return new THREE.MeshStandardMaterial({
          color: planet.color,
          emissive: planet.color,
          emissiveIntensity: 0.3,
          metalness: 0.8,
          roughness: 0.2,
          wireframe: false,
        });
      case "gas":
        return new THREE.MeshStandardMaterial({
          color: planet.color,
          emissive: planet.color,
          emissiveIntensity: 0.15,
          metalness: 0.1,
          roughness: 0.6,
        });
      case "ice":
        return new THREE.MeshPhysicalMaterial({
          color: 0xccddff,
          emissive: planet.color,
          emissiveIntensity: 0.1,
          metalness: 0.2,
          roughness: 0.05,
          transmission: 0.3,
          thickness: 0.5,
          transparent: true,
        });
      case "lava":
        return new THREE.MeshStandardMaterial({
          color: 0x331100,
          emissive: 0xff4400,
          emissiveIntensity: 0.8,
          metalness: 0.4,
          roughness: 0.7,
        });
      case "rocky":
      default:
        return new THREE.MeshStandardMaterial({
          color: planet.color,
          metalness: 0.3,
          roughness: 0.8,
          flatShading: true,
        });
    }
  }

  /** Get the entity ID for a planet. */
  static planetEntityId(planetId: string): string {
    return PLANET_PREFIX + planetId;
  }
}
