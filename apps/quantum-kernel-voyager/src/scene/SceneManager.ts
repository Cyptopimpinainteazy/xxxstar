/**
 * @module scene/SceneManager
 * Central Three.js scene controller.
 *
 * Owns WebGLRenderer, Scene, PerspectiveCamera, render loop,
 * entity registry, and coordinates PostProcessing + CameraController.
 */
import * as THREE from "three";
import type { SceneEntity, RenderStats, VisualConfig } from "../types/scene";
import { DEFAULT_VISUAL_CONFIG } from "../types/scene";
import { PostProcessing } from "./PostProcessing";
import { CameraController } from "./CameraController";
import { LODManager } from "./LODManager";

export class SceneManager {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene: THREE.Scene;
  readonly camera: THREE.PerspectiveCamera;
  readonly cameraController: CameraController;

  private readonly postProcessing: PostProcessing;
  private readonly lodManager: LODManager;
  private readonly entities = new Map<string, { data: SceneEntity; object: THREE.Object3D }>();
  private readonly clock = new THREE.Clock();
  private animationFrameId = 0;
  private running = false;
  private frameCount = 0;
  private lastFpsTime = 0;
  private currentFps = 0;

  private visualConfig: VisualConfig;

  constructor(canvas: HTMLCanvasElement, config: Partial<VisualConfig> = {}) {
    this.visualConfig = { ...DEFAULT_VISUAL_CONFIG, ...config };

    // ---- Renderer ----
    this.renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: true,
      logarithmicDepthBuffer: true,
      powerPreference: "high-performance",
    });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setSize(canvas.clientWidth, canvas.clientHeight);
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.0;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;

    // ---- Scene ----
    this.scene = new THREE.Scene();
    this.scene.fog = new THREE.FogExp2(0x0a0a1a, 0.002);
    this.scene.background = new THREE.Color(0x0a0a1a);

    // ---- Camera ----
    const aspect = canvas.clientWidth / canvas.clientHeight;
    this.camera = new THREE.PerspectiveCamera(60, aspect, 0.1, 10_000);
    this.camera.position.set(0, 30, 50);

    // ---- Lighting ----
    this.setupLighting();

    // ---- Controllers ----
    this.cameraController = new CameraController(this.camera, canvas);
    this.postProcessing = new PostProcessing(this.renderer, this.scene, this.camera, {
      strength: this.visualConfig.bloomStrength,
      radius: 0.5,
      threshold: 0.8,
    });
    this.lodManager = new LODManager(this.camera);

    // ---- Resize observer ----
    const ro = new ResizeObserver(() => this.handleResize());
    ro.observe(canvas);
  }

  // -----------------------------------------------------------------------
  // Entity management
  // -----------------------------------------------------------------------

  addEntity(entity: SceneEntity, object: THREE.Object3D): void {
    this.entities.set(entity.id, { data: entity, object });
    this.scene.add(object);
    this.lodManager.register(entity.id, object);
  }

  removeEntity(id: string): void {
    const entry = this.entities.get(id);
    if (!entry) return;
    this.scene.remove(entry.object);
    this.lodManager.unregister(id);
    disposeObject(entry.object);
    this.entities.delete(id);
  }

  getEntity(id: string): SceneEntity | undefined {
    return this.entities.get(id)?.data;
  }

  getObject(id: string): THREE.Object3D | undefined {
    return this.entities.get(id)?.object;
  }

  getAllEntities(): SceneEntity[] {
    return Array.from(this.entities.values()).map((e) => e.data);
  }

  // -----------------------------------------------------------------------
  // Render loop
  // -----------------------------------------------------------------------

  start(): void {
    if (this.running) return;
    this.running = true;
    this.clock.start();
    this.lastFpsTime = performance.now();
    this.tick();
  }

  stop(): void {
    this.running = false;
    if (this.animationFrameId) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = 0;
    }
  }

  private tick = (): void => {
    if (!this.running) return;
    this.animationFrameId = requestAnimationFrame(this.tick);
    const dt = this.clock.getDelta();

    this.cameraController.update(dt);
    this.lodManager.update();
    this.postProcessing.render();

    // FPS counter
    this.frameCount++;
    const now = performance.now();
    if (now - this.lastFpsTime >= 1000) {
      this.currentFps = this.frameCount;
      this.frameCount = 0;
      this.lastFpsTime = now;
    }
  };

  // -----------------------------------------------------------------------
  // Stats
  // -----------------------------------------------------------------------

  getRenderStats(): RenderStats {
    const info = this.renderer.info;
    return {
      fps: this.currentFps,
      drawCalls: info.render.calls,
      triangles: info.render.triangles,
      entities: this.entities.size,
      geometries: info.memory.geometries,
      textures: info.memory.textures,
      programs: info.render.calls,
      frameTime: this.currentFps > 0 ? 1000 / this.currentFps : 0,
    };
  }

  // -----------------------------------------------------------------------
  // Visual config
  // -----------------------------------------------------------------------

  setVisualConfig(config: Partial<VisualConfig>): void {
    this.visualConfig = { ...this.visualConfig, ...config };
    this.postProcessing.setBloom({ strength: this.visualConfig.bloomStrength });
    if (!this.visualConfig.postProcessingEnabled) {
      this.postProcessing.toggle(false);
    } else {
      this.postProcessing.toggle(true);
    }
  }

  // -----------------------------------------------------------------------
  // Cleanup
  // -----------------------------------------------------------------------

  dispose(): void {
    this.stop();
    for (const [id] of this.entities) {
      this.removeEntity(id);
    }
    this.cameraController.dispose();
    this.postProcessing.dispose();
    this.renderer.dispose();
  }

  // -----------------------------------------------------------------------
  // Private
  // -----------------------------------------------------------------------

  private setupLighting(): void {
    const ambient = new THREE.AmbientLight(0x222244, 0.5);
    this.scene.add(ambient);

    const directional = new THREE.DirectionalLight(0xffffff, 1.0);
    directional.position.set(50, 80, 30);
    directional.castShadow = true;
    directional.shadow.mapSize.width = 2048;
    directional.shadow.mapSize.height = 2048;
    directional.shadow.camera.near = 0.5;
    directional.shadow.camera.far = 200;
    directional.shadow.camera.left = -100;
    directional.shadow.camera.right = 100;
    directional.shadow.camera.top = 100;
    directional.shadow.camera.bottom = -100;
    this.scene.add(directional);

    // Accent point lights for cyberpunk glow
    const cyanPoint = new THREE.PointLight(0x00f0ff, 2, 100);
    cyanPoint.position.set(-20, 15, 20);
    this.scene.add(cyanPoint);

    const magentaPoint = new THREE.PointLight(0xff00ff, 1.5, 80);
    magentaPoint.position.set(20, 10, -15);
    this.scene.add(magentaPoint);
  }

  private handleResize(): void {
    const canvas = this.renderer.domElement;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w === 0 || h === 0) return;
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(w, h, false);
    this.postProcessing.resize(w, h);
  }
}

/** Recursively dispose geometry + materials on an Object3D tree. */
function disposeObject(obj: THREE.Object3D): void {
  obj.traverse((child) => {
    if (child instanceof THREE.Mesh) {
      child.geometry?.dispose();
      if (Array.isArray(child.material)) {
        child.material.forEach((m: THREE.Material) => m.dispose());
      } else {
        child.material?.dispose();
      }
    }
  });
}
