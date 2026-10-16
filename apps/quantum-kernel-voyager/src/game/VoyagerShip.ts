/**
 * @module game/VoyagerShip
 * Player ship control: movement, fuel, health, thruster particles.
 *
 * The ship is represented as a Three.js Group managed by the SceneManager.
 * VoyagerShip coordinates input → physics → position updates.
 */
import * as THREE from "three";
import type { Vec3, ShipState } from "../types/game";
import type { SceneEntity } from "../types/scene";
import { createEntityMesh } from "../scene/EntityFactory";
import type { SceneManager } from "../scene/SceneManager";
import type { ParticleSystem } from "../scene/ParticleSystem";

const SHIP_SPEED = 15;
const SHIP_BOOST_MULTIPLIER = 2.5;
const SHIP_ROTATION_SPEED = 2.0;
const FUEL_DRAIN_RATE = 0.5;       // per second while moving
const BOOST_FUEL_DRAIN = 2.0;      // per second while boosting
const HEALTH_REGEN_RATE = 0.5;     // per second when idle

export class VoyagerShip {
  private readonly entityId = "voyager-ship";
  private readonly sceneManager: SceneManager;
  private readonly particles: ParticleSystem;
  private object: THREE.Group | null = null;

  private velocity = new THREE.Vector3();
  private targetRotation = new THREE.Quaternion();
  private boosting = false;
  private moving = false;
  private maxHealth = 100;

  // Input state
  private input = {
    forward: false,
    backward: false,
    left: false,
    right: false,
    up: false,
    down: false,
    boost: false,
  };

  constructor(sceneManager: SceneManager, particles: ParticleSystem) {
    this.sceneManager = sceneManager;
    this.particles = particles;
  }

  /** Spawn the ship in the scene at a given position. */
  spawn(position: Vec3): void {
    if (this.object) this.despawn();

    this.object = createEntityMesh("voyager_ship", { scale: 1.5 }) as THREE.Group;
    this.object.position.set(position.x, position.y, position.z);

    const entity: SceneEntity = {
      id: this.entityId,
      kind: "voyager_ship",
      position,
      label: "Voyager",
    };

    this.sceneManager.addEntity(entity, this.object);
  }

  /** Remove the ship from the scene. */
  despawn(): void {
    if (this.object) {
      this.sceneManager.removeEntity(this.entityId);
      this.object = null;
    }
  }

  /** Get current world position. */
  getPosition(): Vec3 {
    if (!this.object) return { x: 0, y: 0, z: 0 };
    return {
      x: this.object.position.x,
      y: this.object.position.y,
      z: this.object.position.z,
    };
  }

  /** Get the THREE.Object3D (for camera targeting). */
  getObject(): THREE.Group | null {
    return this.object;
  }

  /** Set ship limits from upgrades. */
  setLimits(_maxFuel: number, maxHealth: number): void {
    this.maxHealth = maxHealth;
  }

  // -----------------------------------------------------------------------
  // Input
  // -----------------------------------------------------------------------

  setInput(key: keyof typeof this.input, value: boolean): void {
    this.input[key] = value;
  }

  // -----------------------------------------------------------------------
  // Update (called every frame)
  // -----------------------------------------------------------------------

  update(dt: number, shipState: ShipState): void {
    if (!this.object) return;

    // Compute desired direction from input
    const direction = new THREE.Vector3();
    if (this.input.forward) direction.z -= 1;
    if (this.input.backward) direction.z += 1;
    if (this.input.left) direction.x -= 1;
    if (this.input.right) direction.x += 1;
    if (this.input.up) direction.y += 1;
    if (this.input.down) direction.y -= 1;

    this.moving = direction.length() > 0;
    this.boosting = this.input.boost && this.moving;

    if (this.moving) {
      direction.normalize();

      // Apply camera-relative rotation
      direction.applyQuaternion(this.object.quaternion);

      const speed = this.boosting ? SHIP_SPEED * SHIP_BOOST_MULTIPLIER : SHIP_SPEED;
      this.velocity.lerp(direction.multiplyScalar(speed), dt * 5);

      // Fuel drain
      const drain = this.boosting ? BOOST_FUEL_DRAIN : FUEL_DRAIN_RATE;
      shipState.fuel = Math.max(0, shipState.fuel - drain * dt);

      // Stop if no fuel
      if (shipState.fuel <= 0) {
        this.velocity.multiplyScalar(0.9);
      }
    } else {
      // Decelerate
      this.velocity.multiplyScalar(1 - dt * 3);

      // Passive health regen
      if (shipState.health < this.maxHealth) {
        shipState.health = Math.min(this.maxHealth, shipState.health + HEALTH_REGEN_RATE * dt);
      }
    }

    // Apply velocity
    this.object.position.addScaledVector(this.velocity, dt);

    // Bank rotation based on lateral velocity
    const bankAngle = -this.velocity.x * 0.02;
    const pitchAngle = this.velocity.z * 0.01;
    this.targetRotation.setFromEuler(new THREE.Euler(pitchAngle, 0, bankAngle));
    this.object.quaternion.slerp(this.targetRotation, dt * SHIP_ROTATION_SPEED);

    // Update ship state position
    shipState.position = this.getPosition();
    shipState.rotation = {
      x: this.object.rotation.x,
      y: this.object.rotation.y,
      z: this.object.rotation.z,
    };

    // Thruster particles
    if (this.moving && shipState.fuel > 0) {
      const thrusterDir = new THREE.Vector3(0, 0, 1).applyQuaternion(this.object.quaternion);
      this.particles.emit(
        "thruster",
        this.object.position.clone().add(thrusterDir.multiplyScalar(1.5)),
        this.boosting ? 4 : 2,
        thrusterDir,
      );
    }
  }

  /** Teleport ship to a position (used after warp). */
  teleport(position: Vec3): void {
    if (!this.object) return;
    this.object.position.set(position.x, position.y, position.z);
    this.velocity.set(0, 0, 0);
  }
}
