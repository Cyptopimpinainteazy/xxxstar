/**
 * @module types/scene
 * 3D scene entity types, visual configuration, and rendering metadata
 * for the Three.js visualization layer.
 */

// ---------------------------------------------------------------------------
// Entity types
// ---------------------------------------------------------------------------

export const SceneEntityKind = {
  BLOCKCHAIN_NODE: "blockchain_node",
  DATA_FLOW: "data_flow",
  CONTRACT: "contract",
  VAULT: "vault",
  PLANET: "planet",
  ARTIFACT: "artifact",
  VOYAGER_SHIP: "voyager_ship",
  WAYPOINT: "waypoint",
} as const;
export type SceneEntityKind = (typeof SceneEntityKind)[keyof typeof SceneEntityKind];

export interface SceneEntity {
  readonly id: string;
  readonly kind: SceneEntityKind;
  readonly label: string;
  readonly position: { x: number; y: number; z: number };
  readonly metadata?: Record<string, unknown>;
  visible?: boolean;
  selected?: boolean;
  hovered?: boolean;
}

export interface BlockchainNodeEntity extends SceneEntity {
  readonly kind: typeof SceneEntityKind.BLOCKCHAIN_NODE;
  readonly blockHeight?: number;
}

export interface DataFlowEntity extends SceneEntity {
  readonly kind: typeof SceneEntityKind.DATA_FLOW;
  readonly metadata: {
    readonly sourceId: string;
    readonly targetId: string;
    readonly messageType: string;
    readonly bandwidth: number;
  };
}

export interface VaultEntity extends SceneEntity {
  readonly kind: typeof SceneEntityKind.VAULT;
  readonly metadata: {
    readonly balance: number;
    readonly currency: string;
    readonly shieldStrength: number;
  };
}

// ---------------------------------------------------------------------------
// Visual config
// ---------------------------------------------------------------------------

export interface BloomConfig {
  readonly strength: number;
  readonly radius: number;
  readonly threshold: number;
}

export interface VisualConfig {
  readonly bloomStrength: number;
  readonly bloomRadius: number;
  readonly bloomThreshold: number;
  readonly starCount: number;
  readonly fxaaEnabled: boolean;
  readonly showGrid: boolean;
  readonly ambientLightIntensity: number;
  readonly backgroundColor: number;
  readonly postProcessingEnabled: boolean;
}

export const DEFAULT_VISUAL_CONFIG: VisualConfig = {
  bloomStrength: 1.5,
  bloomRadius: 0.4,
  bloomThreshold: 0.2,
  starCount: 50_000,
  fxaaEnabled: true,
  showGrid: false,
  ambientLightIntensity: 0.15,
  backgroundColor: 0x0a0a1a,
  postProcessingEnabled: true,
} as const;

// ---------------------------------------------------------------------------
// Camera mode
// ---------------------------------------------------------------------------

export const CameraMode = {
  ORBIT: "orbit",
  FREE_FLIGHT: "free_flight",
} as const;
export type CameraMode = (typeof CameraMode)[keyof typeof CameraMode];

// ---------------------------------------------------------------------------
// Scene export format (for GLTF export via Tauri)
// ---------------------------------------------------------------------------

export interface SceneExport {
  readonly format: "gltf" | "glb";
  readonly sceneJson: string;
  readonly includeTextures: boolean;
}

// ---------------------------------------------------------------------------
// Render statistics (dev tools)
// ---------------------------------------------------------------------------

export interface RenderStats {
  readonly fps: number;
  readonly drawCalls: number;
  readonly triangles: number;
  readonly geometries: number;
  readonly textures: number;
  readonly programs: number;
  readonly entities: number;
  readonly frameTime: number;
}
