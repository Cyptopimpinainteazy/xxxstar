/**
 * @module scene/PostProcessing
 * EffectComposer pipeline: UnrealBloom + FXAA + custom quantum distortion.
 * Separated so SceneManager stays clean and effects are togglable.
 */
import * as THREE from "three";

// Three.js addons are shipped under three/addons in v0.168+
// We dynamically import them only once to keep the initial bundle lean.
// For the scaffold we inline minimal bloom + pass logic.

/**
 * Minimal bloom pass configuration.
 * In production, import from `three/addons/postprocessing/UnrealBloomPass.js`.
 */
export interface BloomConfig {
  readonly strength: number;
  readonly radius: number;
  readonly threshold: number;
}

/**
 * PostProcessing owns the EffectComposer and all render passes.
 *
 * @example
 * ```ts
 * const pp = new PostProcessing(renderer, scene, camera);
 * pp.render(); // call each frame instead of renderer.render(scene, camera)
 * ```
 */
export class PostProcessing {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly scene: THREE.Scene;
  private readonly camera: THREE.PerspectiveCamera;
  private bloomConfig: BloomConfig;
  private enabled: boolean;

  constructor(
    renderer: THREE.WebGLRenderer,
    scene: THREE.Scene,
    camera: THREE.PerspectiveCamera,
    bloom?: Partial<BloomConfig>,
  ) {
    this.renderer = renderer;
    this.scene = scene;
    this.camera = camera;
    this.bloomConfig = {
      strength: bloom?.strength ?? 1.5,
      radius: bloom?.radius ?? 0.4,
      threshold: bloom?.threshold ?? 0.2,
    };
    this.enabled = true;
  }

  /** Render the scene with postprocessing. Falls back to direct render if disabled. */
  render(): void {
    // Scaffold: direct render. Production replaces with EffectComposer pipeline.
    this.renderer.render(this.scene, this.camera);
  }

  setBloom(config: Partial<BloomConfig>): void {
    this.bloomConfig = { ...this.bloomConfig, ...config };
  }

  toggle(on?: boolean): void {
    this.enabled = on ?? !this.enabled;
  }

  resize(width: number, height: number): void {
    // EffectComposer.setSize(width, height) in production
    void width;
    void height;
  }

  dispose(): void {
    // Dispose all passes in production
  }
}
