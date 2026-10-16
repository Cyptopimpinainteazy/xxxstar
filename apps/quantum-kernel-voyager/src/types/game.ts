/**
 * @module types/game
 * Game state, world, artifact, economy, and ship types for the Quantum
 * Kernel Voyager discovery-and-ownership system.
 */

// ---------------------------------------------------------------------------
// Game state machine
// ---------------------------------------------------------------------------

export const GameMode = {
  LOADING: "loading",
  MENU: "menu",
  EXPLORING: "exploring",
  WARPING: "warping",
  INSPECTING: "inspecting",
  PAUSED: "paused",
  SOLAR_SYSTEM: "solar_system",
  LANDING: "landing",
  SURFACE: "surface",
} as const;
export type GameMode = (typeof GameMode)[keyof typeof GameMode];

export interface GameStateSnapshot {
  readonly mode: GameMode;
  readonly timestamp: number;
}

// ---------------------------------------------------------------------------
// World types
// ---------------------------------------------------------------------------

export interface WorldParams {
  readonly seed: string;
  readonly name: string;
  readonly biome: Biome;
  readonly gravity: number;
  readonly atmosphereDensity: number;
  readonly terrainScale: number;
  readonly terrainRoughness: number;
}

export type Biome =
  | "Crystal Fields"
  | "Data Swamp"
  | "Quantum Desert"
  | "Neon Jungle"
  | "Void Tundra"
  | "Plasma Ocean"
  | "Circuit Mountains"
  | "Ether Plains";

export interface DiscoveredWorld {
  readonly seed: string;
  readonly name: string;
  readonly biome: Biome;
  readonly discoveredAt: number;
  artifacts: ArtifactRecord[];
  visited: boolean;
}

// ---------------------------------------------------------------------------
// Artifact types
// ---------------------------------------------------------------------------

export type RarityTier = "Common" | "Uncommon" | "Rare" | "Epic" | "Legendary";

export interface ArtifactRecord {
  readonly id: string;
  readonly name: string;
  readonly rarity: RarityTier;
  readonly worldSeed: string;
  readonly position: Vec3;
  discoveredAt: number;
  readonly geometry: ArtifactGeometry;
  readonly material: ArtifactMaterial;
}

export interface ArtifactGeometry {
  readonly type: string;
  readonly scale: number;
}

export interface ArtifactMaterial {
  readonly baseColor: string;
  readonly emissiveIntensity: number;
  readonly metalness: number;
  readonly roughness: number;
}

// ---------------------------------------------------------------------------
// Ship types
// ---------------------------------------------------------------------------

export interface ShipState {
  position: Vec3;
  rotation: Vec3;
  health: number;
  fuel: number;
  upgrades: ShipUpgrade[];
}

export interface ShipUpgrade {
  readonly id: string;
  readonly name: string;
  readonly description?: string;
  readonly tier?: number;
  readonly category?: "engine" | "hull" | "sensor" | "cosmetic";
  readonly level?: number;
}

// ---------------------------------------------------------------------------
// Economy types
// ---------------------------------------------------------------------------

export interface EconomyState {
  quantumCrystals: number;
  totalMined: number;
  totalSpent: number;
}

// ---------------------------------------------------------------------------
// Common geometry helpers
// ---------------------------------------------------------------------------

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

// ---------------------------------------------------------------------------
// Voyage save format
// ---------------------------------------------------------------------------

export const VOYAGE_SCHEMA_VERSION = 1 as const;

export interface VoyageState {
  readonly schemaVersion: typeof VOYAGE_SCHEMA_VERSION;
  readonly ship: ShipState;
  readonly worlds: DiscoveredWorld[];
  readonly economy: EconomyState;
  readonly savedAt: number;
}
