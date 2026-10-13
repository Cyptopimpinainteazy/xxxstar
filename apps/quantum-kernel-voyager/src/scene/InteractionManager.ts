/**
 * @module scene/InteractionManager
 * Raycaster-based interaction: click, hover, drag, context menu.
 *
 * Manages:
 * - Click-to-select with outline highlight
 * - Hover detection with cursor change
 * - Right-click context menu coordination
 * - Double-click to focus camera on entity
 */
import * as THREE from "three";
import type { SceneEntity } from "../types/scene";

export type InteractionEventType = "select" | "hover" | "deselect" | "context_menu" | "double_click";

export interface InteractionEvent {
  type: InteractionEventType;
  entity: SceneEntity | null;
  /** Screen coordinates of the interaction. */
  screenX: number;
  screenY: number;
  /** World-space hit point (if entity was hit). */
  worldPoint: THREE.Vector3 | null;
}

export type InteractionCallback = (event: InteractionEvent) => void;

export class InteractionManager {
  private readonly camera: THREE.Camera;
  private readonly domElement: HTMLElement;
  private readonly raycaster = new THREE.Raycaster();
  private readonly pointer = new THREE.Vector2();

  /** Map of entity ID → Object3D for raycasting. */
  private readonly targets = new Map<string, { entity: SceneEntity; object: THREE.Object3D }>();
  private hoveredId: string | null = null;
  private selectedId: string | null = null;

  private readonly listeners: InteractionCallback[] = [];

  private readonly onClick: (e: MouseEvent) => void;
  private readonly onDblClick: (e: MouseEvent) => void;
  private readonly onMouseMove: (e: MouseEvent) => void;
  private readonly onContextMenu: (e: MouseEvent) => void;

  constructor(camera: THREE.Camera, domElement: HTMLElement) {
    this.camera = camera;
    this.domElement = domElement;
    this.raycaster.near = 0.1;
    this.raycaster.far = 2000;

    this.onClick = this.handleClick.bind(this);
    this.onDblClick = this.handleDblClick.bind(this);
    this.onMouseMove = this.handleMouseMove.bind(this);
    this.onContextMenu = this.handleContextMenu.bind(this);

    domElement.addEventListener("click", this.onClick);
    domElement.addEventListener("dblclick", this.onDblClick);
    domElement.addEventListener("mousemove", this.onMouseMove);
    domElement.addEventListener("contextmenu", this.onContextMenu);
  }

  /** Register a callback for interaction events. */
  on(callback: InteractionCallback): void {
    this.listeners.push(callback);
  }

  /** Remove a previously registered callback. */
  off(callback: InteractionCallback): void {
    const idx = this.listeners.indexOf(callback);
    if (idx >= 0) this.listeners.splice(idx, 1);
  }

  /** Add an interactive target. */
  register(entity: SceneEntity, object: THREE.Object3D): void {
    this.targets.set(entity.id, { entity, object });
  }

  /** Remove an interactive target. */
  unregister(id: string): void {
    this.targets.delete(id);
    if (this.hoveredId === id) this.hoveredId = null;
    if (this.selectedId === id) this.selectedId = null;
  }

  /** Currently selected entity ID (or null). */
  getSelectedId(): string | null {
    return this.selectedId;
  }

  /** Clear selection. */
  clearSelection(): void {
    if (this.selectedId) {
      this.emit({ type: "deselect", entity: null, screenX: 0, screenY: 0, worldPoint: null });
      this.selectedId = null;
    }
  }

  dispose(): void {
    this.domElement.removeEventListener("click", this.onClick);
    this.domElement.removeEventListener("dblclick", this.onDblClick);
    this.domElement.removeEventListener("mousemove", this.onMouseMove);
    this.domElement.removeEventListener("contextmenu", this.onContextMenu);
    this.listeners.length = 0;
    this.targets.clear();
  }

  // -----------------------------------------------------------------------
  // Private — raycasting
  // -----------------------------------------------------------------------

  private updatePointer(e: MouseEvent): void {
    const rect = this.domElement.getBoundingClientRect();
    this.pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    this.pointer.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
  }

  private raycast(): { entity: SceneEntity; point: THREE.Vector3 } | null {
    this.raycaster.setFromCamera(this.pointer, this.camera);
    const objects = Array.from(this.targets.values()).map((t) => t.object);
    const intersects = this.raycaster.intersectObjects(objects, true);

    if (intersects.length === 0) return null;

    // Walk up to find the registered root object
    let hitObj: THREE.Object3D | null = intersects[0].object;
    while (hitObj) {
      for (const [, target] of this.targets) {
        if (target.object === hitObj) {
          return { entity: target.entity, point: intersects[0].point };
        }
      }
      hitObj = hitObj.parent;
    }
    return null;
  }

  // -----------------------------------------------------------------------
  // Private — event handlers
  // -----------------------------------------------------------------------

  private handleClick(e: MouseEvent): void {
    this.updatePointer(e);
    const hit = this.raycast();

    if (hit) {
      this.selectedId = hit.entity.id;
      this.emit({
        type: "select",
        entity: hit.entity,
        screenX: e.clientX,
        screenY: e.clientY,
        worldPoint: hit.point,
      });
    } else if (this.selectedId) {
      this.selectedId = null;
      this.emit({ type: "deselect", entity: null, screenX: e.clientX, screenY: e.clientY, worldPoint: null });
    }
  }

  private handleDblClick(e: MouseEvent): void {
    this.updatePointer(e);
    const hit = this.raycast();
    if (hit) {
      this.emit({
        type: "double_click",
        entity: hit.entity,
        screenX: e.clientX,
        screenY: e.clientY,
        worldPoint: hit.point,
      });
    }
  }

  private handleMouseMove(e: MouseEvent): void {
    this.updatePointer(e);
    const hit = this.raycast();
    const newHoveredId = hit?.entity.id ?? null;

    if (newHoveredId !== this.hoveredId) {
      // Deselect old hover
      if (this.hoveredId) {
        this.domElement.style.cursor = "default";
      }
      this.hoveredId = newHoveredId;
      if (hit) {
        this.domElement.style.cursor = "pointer";
        this.emit({
          type: "hover",
          entity: hit.entity,
          screenX: e.clientX,
          screenY: e.clientY,
          worldPoint: hit.point,
        });
      }
    }
  }

  private handleContextMenu(e: MouseEvent): void {
    e.preventDefault();
    this.updatePointer(e);
    const hit = this.raycast();
    if (hit) {
      this.emit({
        type: "context_menu",
        entity: hit.entity,
        screenX: e.clientX,
        screenY: e.clientY,
        worldPoint: hit.point,
      });
    }
  }

  private emit(event: InteractionEvent): void {
    for (const cb of this.listeners) {
      cb(event);
    }
  }
}
