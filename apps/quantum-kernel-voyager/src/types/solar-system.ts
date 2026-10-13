/**
 * @module types/solar-system
 * Canonical types for the Atlas Sphere Solar System visualization.
 *
 * The solar system renders the L1 blockchain as a central sun,
 * with tokens, dApps, services, and exchanges as orbiting planets.
 * Swarm nodes are worker avatars on planet surfaces.
 * dApp deployments are factory buildings.
 *
 * These interfaces are LOCKED — do not alter field semantics.
 */

import type { Vec3 } from "./game";

// ---------------------------------------------------------------------------
// Planet types
// ---------------------------------------------------------------------------

export type PlanetType = "token" | "dapp" | "service" | "exchange" | "bridge";

export type SurfaceType = "rocky" | "gas" | "ice" | "lava" | "tech";

/** What the planet "opens" when you land on it. */
export type LandingTargetKind = "internal_route" | "external_url" | "token_detail" | "dapp_detail";

/** A planet in the solar system. */
export interface PlanetEntity {
  readonly id: string;
  readonly name: string;
  readonly type: PlanetType;
  readonly contractAddress?: string;
  readonly chainId?: number;
  /** Current market cap (USD). Drives orbit radius ranking. */
  marketCap: number;
  /** Current price (USD). */
  price: number;
  /** 24h trading volume. */
  volume24h: number;
  /** 24h price change percentage (-100 to +∞). */
  priceChange24h: number;
  readonly iconUrl: string;
  readonly category: string;
  /** Color hex for planet surface/atmosphere tint. */
  readonly color: number;
  readonly hasAtmosphere: boolean;
  readonly surfaceType: SurfaceType;
  /** Where landing takes the user. */
  readonly landingTarget: string;
  readonly landingTargetKind: LandingTargetKind;
  /** Dynamically computed from value ranking. */
  orbitRadius: number;
  /** Dynamically computed from market cap. */
  planetRadius: number;
  /** Orbital angle (radians) — current position around the sun. */
  orbitAngle: number;
  /** Orbital speed (radians per second). */
  orbitSpeed: number;
  /** Orbital inclination (radians). */
  orbitInclination: number;
  /** Active swarm nodes assigned to this planet. */
  swarmNodes: SwarmNodeInfo[];
  /** dApp factories deployed on this planet. */
  factories: FactoryInfo[];
  /** Extensible metadata. */
  metadata: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Swarm node / worker avatar types
// ---------------------------------------------------------------------------

export type SwarmAgentType =
  | "marketing"
  | "development"
  | "security"
  | "analytics"
  | "governance"
  | "gpu-compute";

export type SwarmNodeStatus = "active" | "idle" | "offline" | "error";

export interface SwarmNodeInfo {
  readonly nodeId: string;
  readonly agentType: SwarmAgentType;
  status: SwarmNodeStatus;
  currentTask: string;
  taskProgress: number; // 0–100
  readonly uptime: number; // seconds
  lastHeartbeat: number; // unix ms
  readonly assignedPlanet: string; // planet ID
  position: Vec3;
}

// ---------------------------------------------------------------------------
// Factory / dApp building types
// ---------------------------------------------------------------------------

export type FactoryType = "defi" | "nft" | "gaming" | "utility" | "governance";

export interface FactoryInfo {
  readonly contractAddress: string;
  readonly name: string;
  readonly deployedAt: number; // block number
  readonly type: FactoryType;
  /** Transactions per hour — drives smoke/glow intensity. */
  activity: number;
  /** Lat/lon placement on the planet surface sphere. */
  readonly surfacePosition: { lat: number; lon: number };
}

// ---------------------------------------------------------------------------
// Sun / chain health types
// ---------------------------------------------------------------------------

export interface SunMetrics {
  blockHeight: number;
  blockTime: number; // seconds between blocks
  tps: number; // transactions per second
  validatorCount: number;
  totalStaked: number;
  peerCount: number;
  finalizedHash: string;
  /** 0–1 network health score. Drives sun glow/color. */
  healthScore: number;
}

// ---------------------------------------------------------------------------
// Solar system state
// ---------------------------------------------------------------------------

export type SolarViewMode = "space" | "approaching" | "landing" | "surface";

export interface SolarSystemState {
  viewMode: SolarViewMode;
  selectedPlanetId: string | null;
  targetPlanetId: string | null;
  timeScale: number; // 0 = paused, 1 = normal, 10 = fast
  sunMetrics: SunMetrics;
  planets: PlanetEntity[];
  /** IDs of planets within landing proximity. */
  nearbyPlanetIds: string[];
}

// ---------------------------------------------------------------------------
// Event ripple (for the bottom nav timeline)
// ---------------------------------------------------------------------------

export type RippleEventType = "block" | "transaction" | "deploy" | "governance" | "swarm" | "alert";

export interface EventRipple {
  readonly id: string;
  readonly type: RippleEventType;
  readonly label: string;
  readonly magnitude: number; // 0–1, drives ripple size
  readonly timestamp: number;
  readonly planetId?: string;
}

// ---------------------------------------------------------------------------
// Notification
// ---------------------------------------------------------------------------

export interface SolarNotification {
  readonly id: string;
  readonly message: string;
  readonly type: "info" | "success" | "warning" | "error";
  readonly timestamp: number;
  dismissed: boolean;
}
