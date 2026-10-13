/**
 * @module scene/ProceduralGenerator
 * Deterministic world generation from 256-bit seeds using simplex noise.
 *
 * Generates terrain heightmaps, biome distribution, world names, and
 * artifact placements with rarity tiers.
 */
import { createNoise2D, createNoise3D } from "simplex-noise";
import type { WorldParams, Biome, DiscoveredWorld, ArtifactRecord, RarityTier, Vec3 } from "../types/game";

// -------------------------------------------------------------------------
// Seeded PRNG — xoshiro128** (deterministic from 256-bit seed)
// -------------------------------------------------------------------------

class SeededRng {
  private s: Uint32Array;

  constructor(seed: string) {
    this.s = new Uint32Array(4);
    // Hash seed string into 4 uint32s
    let h = 0x811c9dc5;
    for (let i = 0; i < seed.length; i++) {
      h ^= seed.charCodeAt(i);
      h = Math.imul(h, 0x01000193);
      this.s[i % 4] ^= h >>> 0;
    }
    // Ensure non-zero state
    if (this.s.every((v) => v === 0)) this.s[0] = 1;
    // Warm up
    for (let i = 0; i < 20; i++) this.next();
  }

  /** Returns a float in [0, 1). */
  next(): number {
    const result = this.rotl(Math.imul(this.s[1], 5), 7);
    const t = this.s[1] << 9;
    this.s[2] ^= this.s[0];
    this.s[3] ^= this.s[1];
    this.s[1] ^= this.s[2];
    this.s[0] ^= this.s[3];
    this.s[2] ^= t;
    this.s[3] = this.rotl(this.s[3], 11);
    return (result >>> 0) / 0x100000000;
  }

  /** Returns an int in [min, max). */
  int(min: number, max: number): number {
    return Math.floor(this.next() * (max - min)) + min;
  }

  /** Returns a float in [min, max). */
  float(min: number, max: number): number {
    return this.next() * (max - min) + min;
  }

  private rotl(x: number, k: number): number {
    return ((x << k) | (x >>> (32 - k))) >>> 0;
  }
}

// -------------------------------------------------------------------------
// Name generation
// -------------------------------------------------------------------------

const PREFIXES = [
  "Zo", "Kyr", "Aer", "Ven", "Thal", "Nyx", "Ori", "Xel", "Pyr", "Axa",
  "Lun", "Sol", "Qar", "Dra", "Ith", "Elu", "Vor", "Cyn", "Bel", "Mar",
];
const MIDDLES = [
  "an", "eth", "ion", "aris", "ox", "ul", "em", "ast", "on", "ir",
  "ax", "en", "al", "or", "is", "um", "yx", "el", "ar", "os",
];
const SUFFIXES = [
  "Prime", "Nova", "Core", "Reach", "Deep", "Drift",
  "Gate", "Veil", "Shard", "Forge", "Spire", "Nexus",
];

function generateName(rng: SeededRng): string {
  const prefix = PREFIXES[rng.int(0, PREFIXES.length)];
  const middle = MIDDLES[rng.int(0, MIDDLES.length)];
  const hasSuffix = rng.next() > 0.4;
  const suffix = hasSuffix ? " " + SUFFIXES[rng.int(0, SUFFIXES.length)] : "";
  return prefix + middle + suffix;
}

// -------------------------------------------------------------------------
// Biome selection
// -------------------------------------------------------------------------

// Biome reference list (for procedural selection)
export const ALL_BIOMES: readonly Biome[] = [
  "Crystal Fields", "Data Swamp", "Quantum Desert",
  "Neon Jungle", "Void Tundra", "Plasma Ocean",
  "Circuit Mountains", "Ether Plains",
];

function selectBiome(temperature: number, moisture: number): Biome {
  // Map temperature/moisture combos to biomes
  if (temperature < -0.3) {
    return moisture > 0 ? "Void Tundra" : "Quantum Desert";
  }
  if (temperature > 0.3) {
    if (moisture > 0.3) return "Neon Jungle";
    if (moisture > 0) return "Plasma Ocean";
    return "Crystal Fields";
  }
  if (moisture > 0.2) return "Data Swamp";
  if (moisture < -0.2) return "Circuit Mountains";
  return "Ether Plains";
}

// -------------------------------------------------------------------------
// Rarity roll
// -------------------------------------------------------------------------

function rollRarity(rng: SeededRng): RarityTier {
  const r = rng.next();
  if (r < 0.01) return "Legendary";   // 1%
  if (r < 0.05) return "Epic";        // 4%
  if (r < 0.15) return "Rare";        // 10%
  if (r < 0.40) return "Uncommon";    // 25%
  return "Common";                     // 60%
}

// -------------------------------------------------------------------------
// Heightmap
// -------------------------------------------------------------------------

export interface HeightmapResult {
  /** Row-major float array, values in [0, 1]. */
  data: Float32Array;
  width: number;
  height: number;
}

/**
 * Generate a deterministic heightmap from world params.
 * Uses 6-octave simplex noise.
 */
export function generateHeightmap(params: WorldParams, resolution = 128): HeightmapResult {
  const rng = new SeededRng(params.seed);
  const offsetX = rng.float(-1000, 1000);
  const offsetZ = rng.float(-1000, 1000);
  const noise2d = createNoise2D(() => rng.next());

  const data = new Float32Array(resolution * resolution);
  const octaves = 6;
  const lacunarity = params.terrainRoughness * 2 + 1;
  const persistence = 0.5;

  for (let z = 0; z < resolution; z++) {
    for (let x = 0; x < resolution; x++) {
      let amplitude = 1;
      let frequency = 0.02;
      let value = 0;
      let maxAmp = 0;

      for (let o = 0; o < octaves; o++) {
        const nx = (x + offsetX) * frequency;
        const nz = (z + offsetZ) * frequency;
        value += noise2d(nx, nz) * amplitude;
        maxAmp += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
      }

      // Normalize to [0, 1]
      value = (value / maxAmp + 1) / 2;
      // Apply terrain scale
      value *= params.terrainScale;
      data[z * resolution + x] = value;
    }
  }

  return { data, width: resolution, height: resolution };
}

// -------------------------------------------------------------------------
// Full world generation
// -------------------------------------------------------------------------

/**
 * Generate a complete world definition from a seed string.
 */
export function generateWorld(seed: string): { world: DiscoveredWorld; params: WorldParams } {
  const rng = new SeededRng(seed);

  const biomeTemp = rng.float(-1, 1);
  const biomeMoist = rng.float(-1, 1);
  const biome = selectBiome(biomeTemp, biomeMoist);

  const params: WorldParams = {
    seed,
    name: generateName(rng),
    biome,
    gravity: rng.float(0.3, 2.5),
    atmosphereDensity: rng.float(0, 1),
    terrainScale: rng.float(5, 50),
    terrainRoughness: rng.float(0.2, 0.8),
  };

  const world: DiscoveredWorld = {
    seed,
    name: params.name,
    biome,
    discoveredAt: Date.now(),
    artifacts: [],
    visited: false,
  };

  return { world, params };
}

// -------------------------------------------------------------------------
// Artifact placement
// -------------------------------------------------------------------------

/**
 * Generate artifacts for a world. Positions are in world-space units.
 */
export function generateArtifacts(
  seed: string,
  worldRadius: number,
  count: number,
): ArtifactRecord[] {
  const rng = new SeededRng(seed + ":artifacts");
  const noise3d = createNoise3D(() => rng.next());
  const artifacts: ArtifactRecord[] = [];

  for (let i = 0; i < count; i++) {
    // Scatter on planet surface
    const theta = rng.float(0, Math.PI * 2);
    const phi = Math.acos(rng.float(-1, 1));
    const r = worldRadius * 1.01; // Slightly above surface
    const position: Vec3 = {
      x: r * Math.sin(phi) * Math.cos(theta),
      y: r * Math.sin(phi) * Math.sin(theta),
      z: r * Math.cos(phi),
    };

    const rarity = rollRarity(rng);
    const noiseVal = noise3d(position.x * 0.1, position.y * 0.1, position.z * 0.1);

    artifacts.push({
      id: `artifact-${seed}-${i}`,
      name: generateArtifactName(rng, rarity),
      rarity,
      discoveredAt: 0,
      worldSeed: seed,
      position,
      geometry: {
        type: noiseVal > 0 ? "crystal" : "orb",
        scale: rarity === "Legendary" ? 2.0 : rarity === "Epic" ? 1.5 : 1.0,
      },
      material: {
        baseColor: rarityHex(rarity),
        emissiveIntensity: rarity === "Legendary" ? 1.0 : rarity === "Epic" ? 0.7 : 0.3,
        metalness: 0.5,
        roughness: 0.1,
      },
    });
  }

  return artifacts;
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

const ARTIFACT_ADJECTIVES = [
  "Quantum", "Prismatic", "Void", "Stellar", "Ancient",
  "Fractal", "Harmonic", "Ethereal", "Temporal", "Radiant",
];
const ARTIFACT_NOUNS = [
  "Shard", "Core", "Lens", "Key", "Matrix",
  "Prism", "Relic", "Cipher", "Beacon", "Conduit",
];

function generateArtifactName(rng: SeededRng, _rarity: RarityTier): string {
  const adj = ARTIFACT_ADJECTIVES[rng.int(0, ARTIFACT_ADJECTIVES.length)];
  const noun = ARTIFACT_NOUNS[rng.int(0, ARTIFACT_NOUNS.length)];
  return `${adj} ${noun}`;
}

function rarityHex(r: RarityTier): string {
  const map: Record<RarityTier, string> = {
    Common: "#888888",
    Uncommon: "#00ff88",
    Rare: "#00f0ff",
    Epic: "#ff00ff",
    Legendary: "#ffd700",
  };
  return map[r];
}
