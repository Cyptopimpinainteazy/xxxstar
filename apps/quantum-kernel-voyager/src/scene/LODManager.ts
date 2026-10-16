/**
 * @module scene/LODManager
 * Three-level LOD, frustum culling, and GPU memory budget tracking.
 *
 * - LOD 0 (< 30 units): Full detail
 * - LOD 1 (30–100 units): Reduced geometry / simpler materials
 * - LOD 2 (> 100 units): Billboard or hidden
 *
 * Warns when GPU memory usage exceeds 512 MB budget.
 */
import * as THREE from "three";

const LOD_NEAR = 30;
const LOD_MID = 100;
const GPU_BUDGET_MB = 512;

interface LODEntry {
  object: THREE.Object3D;
  /** Original full-detail scale. */
  baseScale: THREE.Vector3;
  currentLOD: 0 | 1 | 2;
  visible: boolean;
}

export class LODManager {
  private readonly camera: THREE.Camera;
  private readonly entries = new Map<string, LODEntry>();
  private readonly frustum = new THREE.Frustum();
  private readonly projScreenMatrix = new THREE.Matrix4();
  private readonly tempVec = new THREE.Vector3();

  private gpuMemoryWarningIssued = false;

  constructor(camera: THREE.Camera) {
    this.camera = camera;
  }

  /** Register an entity's root Object3D for LOD management. */
  register(id: string, object: THREE.Object3D): void {
    this.entries.set(id, {
      object,
      baseScale: object.scale.clone(),
      currentLOD: 0,
      visible: true,
    });
  }

  /** Unregister an entity. */
  unregister(id: string): void {
    this.entries.delete(id);
  }

  /** Must be called every frame to update LOD and culling. */
  update(): void {
    // Rebuild frustum from camera
    this.projScreenMatrix.multiplyMatrices(
      (this.camera as THREE.PerspectiveCamera).projectionMatrix,
      this.camera.matrixWorldInverse,
    );
    this.frustum.setFromProjectionMatrix(this.projScreenMatrix);

    for (const entry of this.entries.values()) {
      this.updateEntry(entry);
    }
  }

  /** Estimate current GPU memory usage in MB (approximate). */
  estimateGpuMemoryMb(rendererInfo: THREE.WebGLInfo): number {
    const geoMb = rendererInfo.memory.geometries * 0.01; // Rough average
    const texMb = rendererInfo.memory.textures * 0.5;    // Rough average
    return geoMb + texMb;
  }

  /** Check memory budget. Returns true if over budget. */
  checkBudget(rendererInfo: THREE.WebGLInfo): boolean {
    const usage = this.estimateGpuMemoryMb(rendererInfo);
    if (usage > GPU_BUDGET_MB && !this.gpuMemoryWarningIssued) {
      console.warn(
        `[LODManager] GPU memory estimate ${usage.toFixed(1)} MB exceeds budget ${GPU_BUDGET_MB} MB`,
      );
      this.gpuMemoryWarningIssued = true;
      return true;
    }
    if (usage <= GPU_BUDGET_MB * 0.9) {
      this.gpuMemoryWarningIssued = false;
    }
    return usage > GPU_BUDGET_MB;
  }

  /** Current entity count by LOD level. */
  getStats(): { lod0: number; lod1: number; lod2: number; culled: number } {
    let lod0 = 0;
    let lod1 = 0;
    let lod2 = 0;
    let culled = 0;
    for (const entry of this.entries.values()) {
      if (!entry.visible) { culled++; continue; }
      switch (entry.currentLOD) {
        case 0: lod0++; break;
        case 1: lod1++; break;
        case 2: lod2++; break;
      }
    }
    return { lod0, lod1, lod2, culled };
  }

  // -----------------------------------------------------------------------
  // Private
  // -----------------------------------------------------------------------

  private updateEntry(entry: LODEntry): void {
    const obj = entry.object;

    // Frustum culling
    obj.getWorldPosition(this.tempVec);
    const inFrustum = this.frustum.containsPoint(this.tempVec);

    if (!inFrustum) {
      if (entry.visible) {
        obj.visible = false;
        entry.visible = false;
      }
      return;
    }

    if (!entry.visible) {
      obj.visible = true;
      entry.visible = true;
    }

    // Distance-based LOD
    const camPos = this.camera.getWorldPosition(new THREE.Vector3());
    const distance = camPos.distanceTo(this.tempVec);
    let newLOD: 0 | 1 | 2;

    if (distance < LOD_NEAR) {
      newLOD = 0;
    } else if (distance < LOD_MID) {
      newLOD = 1;
    } else {
      newLOD = 2;
    }

    if (newLOD !== entry.currentLOD) {
      this.applyLOD(entry, newLOD);
      entry.currentLOD = newLOD;
    }
  }

  private applyLOD(entry: LODEntry, level: 0 | 1 | 2): void {
    const obj = entry.object;

    switch (level) {
      case 0:
        // Full detail
        obj.scale.copy(entry.baseScale);
        this.setChildrenDetail(obj, true);
        break;
      case 1:
        // Reduced: hide secondary children (wireframes, particles, rings)
        obj.scale.copy(entry.baseScale);
        this.setChildrenDetail(obj, false);
        break;
      case 2:
        // Minimal: scale down, hide all decorative children
        obj.scale.copy(entry.baseScale).multiplyScalar(0.5);
        this.setChildrenDetail(obj, false);
        break;
    }
  }

  /**
   * Toggle visibility of secondary/decorative child meshes.
   * The first child (primary mesh) is always visible.
   */
  private setChildrenDetail(obj: THREE.Object3D, detailed: boolean): void {
    const children = obj.children;
    for (let i = 1; i < children.length; i++) {
      children[i].visible = detailed;
    }
  }
}
