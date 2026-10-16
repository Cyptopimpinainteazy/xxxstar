/**
 * @module game/ArtifactManager
 * Discovery, inventory, and collection tracking for artifacts.
 *
 * Handles scanning, claiming, and displaying artifact details.
 * Coordinates with EconomyManager for crystal yields.
 */
import type { ArtifactRecord, RarityTier, Vec3 } from "../types/game";
import type { EconomyManager } from "./EconomyManager";

export interface ArtifactScanResult {
  artifact: ArtifactRecord;
  distance: number;
  canClaim: boolean;
}

const CLAIM_RADIUS = 10;

export class ArtifactManager {
  private readonly inventory: ArtifactRecord[] = [];
  private readonly worldArtifacts = new Map<string, ArtifactRecord[]>();

  constructor(private readonly economy: EconomyManager) {}

  /** Register artifacts generated for a world. */
  setWorldArtifacts(worldSeed: string, artifacts: ArtifactRecord[]): void {
    this.worldArtifacts.set(worldSeed, [...artifacts]);
  }

  /** Remove a world's artifacts on unload. */
  clearWorldArtifacts(worldSeed: string): void {
    this.worldArtifacts.delete(worldSeed);
  }

  /** Get artifacts for the currently loaded world. */
  getWorldArtifacts(worldSeed: string): ArtifactRecord[] {
    return this.worldArtifacts.get(worldSeed) ?? [];
  }

  /** Scan for nearby artifacts from a position. Returns sorted by distance. */
  scan(worldSeed: string, position: Vec3, maxRange: number): ArtifactScanResult[] {
    const artifacts = this.worldArtifacts.get(worldSeed) ?? [];
    const results: ArtifactScanResult[] = [];

    for (const art of artifacts) {
      // Skip already claimed
      if (art.discoveredAt > 0) continue;

      const d = dist(position, art.position);
      if (d > maxRange) continue;

      results.push({
        artifact: art,
        distance: d,
        canClaim: d <= CLAIM_RADIUS,
      });
    }

    results.sort((a, b) => a.distance - b.distance);
    return results;
  }

  /**
   * Claim an artifact. Must be within CLAIM_RADIUS.
   * Awards crystals via EconomyManager.
   * @returns crystals earned, or 0 if already claimed / out of range
   */
  claim(worldSeed: string, artifactId: string, shipPos: Vec3): number {
    const artifacts = this.worldArtifacts.get(worldSeed);
    if (!artifacts) return 0;

    const art = artifacts.find((a) => a.id === artifactId);
    if (!art || art.discoveredAt > 0) return 0;

    const d = dist(shipPos, art.position);
    if (d > CLAIM_RADIUS) return 0;

    art.discoveredAt = Date.now();
    this.inventory.push({ ...art });

    return this.economy.mineArtifact(art.rarity);
  }

  /** Full inventory (all claimed artifacts across all worlds). */
  getInventory(): ArtifactRecord[] {
    return [...this.inventory];
  }

  /** Filter inventory by rarity. */
  getInventoryByRarity(rarity: RarityTier): ArtifactRecord[] {
    return this.inventory.filter((a) => a.rarity === rarity);
  }

  /** Count artifacts by rarity tier. */
  getInventoryStats(): Record<RarityTier, number> {
    const stats: Record<RarityTier, number> = {
      Common: 0,
      Uncommon: 0,
      Rare: 0,
      Epic: 0,
      Legendary: 0,
    };
    for (const art of this.inventory) {
      stats[art.rarity]++;
    }
    return stats;
  }

  /** Total claimed count. */
  totalClaimed(): number {
    return this.inventory.length;
  }

  /** Restore inventory from a saved list. */
  loadInventory(items: ArtifactRecord[]): void {
    this.inventory.length = 0;
    this.inventory.push(...items);
  }
}

// -------------------------------------------------------------------------
// Utility
// -------------------------------------------------------------------------

function dist(a: Vec3, b: Vec3): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  const dz = a.z - b.z;
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}
