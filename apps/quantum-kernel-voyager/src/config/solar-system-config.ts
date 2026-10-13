/**
 * @module config/solar-system-config
 * Configuration constants and the initial planet registry for the
 * Atlas Sphere Solar System visualization.
 *
 * The registry defines the 10 core ecosystem planets that are always
 * present. Additional token/dApp planets are added dynamically from
 * on-chain data.
 */
import type { PlanetEntity, SunMetrics } from "../types/solar-system";

// ---------------------------------------------------------------------------
// Orbit / sizing constants
// ---------------------------------------------------------------------------

/** Minimum orbit radius (closest planet to sun). */
export const MIN_ORBIT_RADIUS = 30;
/** Maximum orbit radius (furthest planet from sun). */
export const MAX_ORBIT_RADIUS = 200;

/** Minimum planet visual radius. */
export const MIN_PLANET_RADIUS = 1.5;
/** Maximum planet visual radius. */
export const MAX_PLANET_RADIUS = 8;

/** Sun visual radius. */
export const SUN_RADIUS = 12;

/** Landing proximity threshold (multiple of planet radius). */
export const LANDING_PROXIMITY_MULTIPLIER = 3.5;

/** How fast orbits move at 1x time scale. */
export const BASE_ORBIT_SPEED = 0.02; // radians per second at radius = MIN_ORBIT_RADIUS

// ---------------------------------------------------------------------------
// Default sun metrics (simulated when no RPC available)
// ---------------------------------------------------------------------------

export const DEFAULT_SUN_METRICS: SunMetrics = {
  blockHeight: 0,
  blockTime: 6.0,
  tps: 0,
  validatorCount: 12,
  totalStaked: 1_000_000,
  peerCount: 8,
  finalizedHash: "0x0000000000000000000000000000000000000000000000000000000000000000",
  healthScore: 0.85,
};

// ---------------------------------------------------------------------------
// Core ecosystem planet registry
// ---------------------------------------------------------------------------

function planet(
  id: string,
  name: string,
  type: PlanetEntity["type"],
  opts: {
    color: number;
    surfaceType: PlanetEntity["surfaceType"];
    landingTarget: string;
    landingTargetKind: PlanetEntity["landingTargetKind"];
    category: string;
    marketCap?: number;
    hasAtmosphere?: boolean;
    orbitInclination?: number;
  },
): PlanetEntity {
  return {
    id,
    name,
    type,
    color: opts.color,
    surfaceType: opts.surfaceType,
    landingTarget: opts.landingTarget,
    landingTargetKind: opts.landingTargetKind,
    category: opts.category,
    iconUrl: "",
    marketCap: opts.marketCap ?? 0,
    price: 0,
    volume24h: 0,
    priceChange24h: 0,
    hasAtmosphere: opts.hasAtmosphere ?? true,
    orbitRadius: 0,   // computed at runtime
    planetRadius: 0,  // computed at runtime
    orbitAngle: Math.random() * Math.PI * 2,
    orbitSpeed: 0,    // computed at runtime
    orbitInclination: opts.orbitInclination ?? 0,
    swarmNodes: [],
    factories: [],
    metadata: {},
  };
}

/**
 * The 10 core ecosystem planets. Order = initial value ranking.
 * Orbit radii and sizes are computed at runtime by OrbitCalculator.
 */
export const CORE_PLANETS: PlanetEntity[] = [
  // Rank 1: Atlas Sphere Token (closest to sun)
  planet("atlas-token", "ATLAS Token", "token", {
    color: 0x00f0ff,
    surfaceType: "tech",
    landingTarget: "/token/atlas",
    landingTargetKind: "token_detail",
    category: "Native Token",
    marketCap: 500_000_000,
    orbitInclination: 0.05,
  }),

  // Rank 2: Block Explorer
  planet("block-explorer", "Block Explorer", "service", {
    color: 0x4488ff,
    surfaceType: "tech",
    landingTarget: "/explorer",
    landingTargetKind: "internal_route",
    category: "Infrastructure",
    marketCap: 100_000_000,
  }),

  // Rank 3: DEX / Swap
  planet("atlas-dex", "Atlas DEX", "exchange", {
    color: 0xffd700,
    surfaceType: "gas",
    landingTarget: "/swap",
    landingTargetKind: "internal_route",
    category: "DeFi",
    marketCap: 80_000_000,
    orbitInclination: 0.12,
  }),

  // Rank 4: Swarm Dashboard
  planet("swarm-hub", "Swarm Hub", "service", {
    color: 0x00ff88,
    surfaceType: "tech",
    landingTarget: "/x3/swarm",
    landingTargetKind: "internal_route",
    category: "Infrastructure",
    marketCap: 60_000_000,
    orbitInclination: -0.08,
  }),

  // Rank 5: GPU Compute Cluster
  planet("gpu-cluster", "GPU Cluster", "service", {
    color: 0xff6600,
    surfaceType: "lava",
    landingTarget: "/x3/swarm/gpu",
    landingTargetKind: "internal_route",
    category: "Compute",
    marketCap: 50_000_000,
    orbitInclination: 0.15,
  }),

  // Rank 6: Wallet
  planet("atlas-wallet", "Wallet", "service", {
    color: 0x8866ff,
    surfaceType: "ice",
    landingTarget: "/wallet",
    landingTargetKind: "internal_route",
    category: "Finance",
    marketCap: 40_000_000,
    orbitInclination: -0.05,
  }),

  // Rank 7: Governance
  planet("governance", "Governance", "service", {
    color: 0xff00ff,
    surfaceType: "rocky",
    landingTarget: "/governance",
    landingTargetKind: "internal_route",
    category: "Governance",
    marketCap: 30_000_000,
    orbitInclination: 0.2,
  }),

  // Rank 8: Treasury
  planet("treasury", "Treasury", "service", {
    color: 0xffd700,
    surfaceType: "rocky",
    landingTarget: "/treasury",
    landingTargetKind: "internal_route",
    category: "Finance",
    marketCap: 25_000_000,
    hasAtmosphere: false,
    orbitInclination: -0.1,
  }),

  // Rank 9: Analytics
  planet("analytics", "Analytics", "service", {
    color: 0x44ddff,
    surfaceType: "gas",
    landingTarget: "/analytics",
    landingTargetKind: "internal_route",
    category: "Intelligence",
    marketCap: 15_000_000,
    orbitInclination: 0.1,
  }),

  // Rank 10: Bridge (Ethereum)
  planet("eth-bridge", "ETH Bridge", "bridge", {
    color: 0x6688cc,
    surfaceType: "ice",
    landingTarget: "https://bridge.atlas-sphere.io",
    landingTargetKind: "external_url",
    category: "Bridge",
    marketCap: 10_000_000,
    orbitInclination: -0.18,
  }),
];
