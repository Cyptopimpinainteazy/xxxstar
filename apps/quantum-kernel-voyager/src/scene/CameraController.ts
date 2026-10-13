/**
 * @module scene/CameraController
 * Orbit + free-flight camera with smooth animated transitions.
 *
 * - Orbit mode: azimuth/polar rotation, dolly zoom, pan
 * - Free-flight mode: WASD + mouse-look
 * - Transition between modes with 500ms eased animation
 * - Click-to-focus: smooth fly-to target position
 */
import * as THREE from "three";
import type { CameraMode } from "../types/scene";

interface KeyState {
  forward: boolean;
  backward: boolean;
  left: boolean;
  right: boolean;
  up: boolean;
  down: boolean;
}

/**
 * Unified camera controller supporting orbit and free-flight modes.
 *
 * @example
 * ```ts
 * const cc = new CameraController(camera, canvas);
 * cc.setMode("orbit");
 * // in render loop:
 * cc.update(deltaTime);
 * ```
 */
export class CameraController {
  private readonly camera: THREE.PerspectiveCamera;
  private readonly domElement: HTMLElement;
  private mode: CameraMode = "orbit";

  // Orbit state
  private spherical = new THREE.Spherical(50, Math.PI / 3, 0);
  private target = new THREE.Vector3(0, 0, 0);
  private panOffset = new THREE.Vector3();

  // Free-flight state
  private euler = new THREE.Euler(0, 0, 0, "YXZ");
  private readonly moveSpeed = 20;
  private readonly lookSensitivity = 0.002;

  // Transition
  private transitioning = false;
  private transitionStart = 0;
  private transitionDuration = 500;
  private transitionFrom = new THREE.Vector3();
  private transitionTo = new THREE.Vector3();
  private transitionTargetFrom = new THREE.Vector3();
  private transitionTargetTo = new THREE.Vector3();

  // Input
  private keys: KeyState = { forward: false, backward: false, left: false, right: false, up: false, down: false };
  private pointerDown = false;
  private lastPointerX = 0;
  private lastPointerY = 0;

  private readonly onKeyDown: (e: KeyboardEvent) => void;
  private readonly onKeyUp: (e: KeyboardEvent) => void;
  private readonly onPointerDown: (e: PointerEvent) => void;
  private readonly onPointerUp: (e: PointerEvent) => void;
  private readonly onPointerMove: (e: PointerEvent) => void;
  private readonly onWheel: (e: WheelEvent) => void;

  constructor(camera: THREE.PerspectiveCamera, domElement: HTMLElement) {
    this.camera = camera;
    this.domElement = domElement;

    this.onKeyDown = this.handleKeyDown.bind(this);
    this.onKeyUp = this.handleKeyUp.bind(this);
    this.onPointerDown = this.handlePointerDown.bind(this);
    this.onPointerUp = this.handlePointerUp.bind(this);
    this.onPointerMove = this.handlePointerMove.bind(this);
    this.onWheel = this.handleWheel.bind(this);

    window.addEventListener("keydown", this.onKeyDown);
    window.addEventListener("keyup", this.onKeyUp);
    domElement.addEventListener("pointerdown", this.onPointerDown);
    window.addEventListener("pointerup", this.onPointerUp);
    window.addEventListener("pointermove", this.onPointerMove);
    domElement.addEventListener("wheel", this.onWheel, { passive: false });

    this.applyOrbit();
  }

  /** Current camera mode. */
  getMode(): CameraMode {
    return this.mode;
  }

  /** Switch modes with optional animated transition. */
  setMode(mode: CameraMode, animate = true): void {
    if (mode === this.mode) return;
    if (animate) {
      this.startTransition(this.camera.position.clone(), this.camera.position.clone());
    }
    this.mode = mode;
    if (mode === "free_flight") {
      this.euler.setFromQuaternion(this.camera.quaternion);
    }
  }

  /** Smoothly fly camera to focus on a world-space position. */
  focusOn(position: THREE.Vector3, distance = 15): void {
    const dir = new THREE.Vector3(1, 0.5, 1).normalize();
    const dest = position.clone().add(dir.multiplyScalar(distance));
    this.startTransition(dest, position);
  }

  /** Must be called every frame. */
  update(dt: number): void {
    if (this.transitioning) {
      this.updateTransition();
      return;
    }
    if (this.mode === "orbit") {
      this.applyOrbit();
    } else {
      this.applyFreeFlight(dt);
    }
  }

  dispose(): void {
    window.removeEventListener("keydown", this.onKeyDown);
    window.removeEventListener("keyup", this.onKeyUp);
    this.domElement.removeEventListener("pointerdown", this.onPointerDown);
    window.removeEventListener("pointerup", this.onPointerUp);
    window.removeEventListener("pointermove", this.onPointerMove);
    this.domElement.removeEventListener("wheel", this.onWheel);
  }

  // -----------------------------------------------------------------------
  // Private — orbit
  // -----------------------------------------------------------------------

  private applyOrbit(): void {
    const pos = new THREE.Vector3().setFromSpherical(this.spherical);
    pos.add(this.target).add(this.panOffset);
    this.camera.position.copy(pos);
    this.camera.lookAt(this.target.clone().add(this.panOffset));
  }

  // -----------------------------------------------------------------------
  // Private — free-flight
  // -----------------------------------------------------------------------

  private applyFreeFlight(dt: number): void {
    const direction = new THREE.Vector3();
    if (this.keys.forward) direction.z -= 1;
    if (this.keys.backward) direction.z += 1;
    if (this.keys.left) direction.x -= 1;
    if (this.keys.right) direction.x += 1;
    if (this.keys.up) direction.y += 1;
    if (this.keys.down) direction.y -= 1;
    direction.normalize().multiplyScalar(this.moveSpeed * dt);

    direction.applyQuaternion(this.camera.quaternion);
    this.camera.position.add(direction);
  }

  // -----------------------------------------------------------------------
  // Private — transitions
  // -----------------------------------------------------------------------

  private startTransition(toPos: THREE.Vector3, toTarget: THREE.Vector3): void {
    this.transitioning = true;
    this.transitionStart = performance.now();
    this.transitionFrom.copy(this.camera.position);
    this.transitionTo.copy(toPos);
    this.transitionTargetFrom.copy(this.target);
    this.transitionTargetTo.copy(toTarget);
  }

  private updateTransition(): void {
    const elapsed = performance.now() - this.transitionStart;
    let t = Math.min(elapsed / this.transitionDuration, 1);
    // Ease in-out cubic
    t = t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;

    this.camera.position.lerpVectors(this.transitionFrom, this.transitionTo, t);
    const lookTarget = new THREE.Vector3().lerpVectors(this.transitionTargetFrom, this.transitionTargetTo, t);
    this.camera.lookAt(lookTarget);

    if (t >= 1) {
      this.transitioning = false;
      this.target.copy(this.transitionTargetTo);
      if (this.mode === "orbit") {
        this.spherical.setFromVector3(this.camera.position.clone().sub(this.target));
      }
    }
  }

  // -----------------------------------------------------------------------
  // Private — input handlers
  // -----------------------------------------------------------------------

  private handleKeyDown(e: KeyboardEvent): void {
    // Don't capture if user is typing in an input
    if ((e.target as HTMLElement).tagName === "INPUT" || (e.target as HTMLElement).tagName === "TEXTAREA") return;
    switch (e.code) {
      case "KeyW": this.keys.forward = true; break;
      case "KeyS": this.keys.backward = true; break;
      case "KeyA": this.keys.left = true; break;
      case "KeyD": this.keys.right = true; break;
      case "Space": this.keys.up = true; e.preventDefault(); break;
      case "ShiftLeft": this.keys.down = true; break;
      case "KeyF": this.setMode(this.mode === "orbit" ? "free_flight" : "orbit"); break;
    }
  }

  private handleKeyUp(e: KeyboardEvent): void {
    switch (e.code) {
      case "KeyW": this.keys.forward = false; break;
      case "KeyS": this.keys.backward = false; break;
      case "KeyA": this.keys.left = false; break;
      case "KeyD": this.keys.right = false; break;
      case "Space": this.keys.up = false; break;
      case "ShiftLeft": this.keys.down = false; break;
    }
  }

  private handlePointerDown(e: PointerEvent): void {
    this.pointerDown = true;
    this.lastPointerX = e.clientX;
    this.lastPointerY = e.clientY;
  }

  private handlePointerUp(_e: PointerEvent): void {
    this.pointerDown = false;
  }

  private handlePointerMove(e: PointerEvent): void {
    if (!this.pointerDown) return;
    const dx = e.clientX - this.lastPointerX;
    const dy = e.clientY - this.lastPointerY;
    this.lastPointerX = e.clientX;
    this.lastPointerY = e.clientY;

    if (this.mode === "orbit") {
      this.spherical.theta -= dx * 0.005;
      this.spherical.phi = Math.max(0.1, Math.min(Math.PI - 0.1, this.spherical.phi - dy * 0.005));
    } else {
      this.euler.y -= dx * this.lookSensitivity;
      this.euler.x -= dy * this.lookSensitivity;
      this.euler.x = Math.max(-Math.PI / 2, Math.min(Math.PI / 2, this.euler.x));
      this.camera.quaternion.setFromEuler(this.euler);
    }
  }

  private handleWheel(e: WheelEvent): void {
    e.preventDefault();
    if (this.mode === "orbit") {
      this.spherical.radius = Math.max(5, Math.min(500, this.spherical.radius + e.deltaY * 0.05));
    }
  }
}
