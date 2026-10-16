/**
 * @module game/EconomyManager
 * Quantum Crystals economy: mining, spending, rewards, and conversion.
 *
 * Economic rules:
 * - Mining: artifacts yield crystals based on rarity
 * - Fuel: warp costs scale with distance
 * - Upgrades: ship improvements purchased with crystals
 * - Block rewards: passive crystals from staking (chain events)
 */
import type { RarityTier, ShipUpgrade, EconomyState } from "../types/game";
import type { GameState } from "./GameState";

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

const RARITY_YIELD: Record<RarityTier, number> = {
  Common: 10,
  Uncommon: 25,
  Rare: 75,
  Epic: 200,
  Legendary: 1000,
};

const WARP_BASE_COST = 5;
const WARP_DISTANCE_FACTOR = 0.1;

const UPGRADE_CATALOG: Record<string, ShipUpgrade & { cost: number }> = {
  fuel_tank_mk2: {
    id: "fuel_tank_mk2",
    name: "Fuel Tank Mk2",
    description: "Doubles maximum fuel capacity.",
    tier: 1,
    cost: 100,
  },
  fuel_tank_mk3: {
    id: "fuel_tank_mk3",
    name: "Fuel Tank Mk3",
    description: "Triples maximum fuel capacity.",
    tier: 2,
    cost: 500,
  },
  scanner_mk2: {
    id: "scanner_mk2",
    name: "Deep Scanner Mk2",
    description: "Reveals uncommon+ artifacts from orbit.",
    tier: 1,
    cost: 150,
  },
  scanner_mk3: {
    id: "scanner_mk3",
    name: "Quantum Scanner Mk3",
    description: "Reveals all artifacts and hidden worlds.",
    tier: 2,
    cost: 750,
  },
  hull_reinforced: {
    id: "hull_reinforced",
    name: "Reinforced Hull",
    description: "+50 max health.",
    tier: 1,
    cost: 200,
  },
  warp_drive_mk2: {
    id: "warp_drive_mk2",
    name: "Warp Drive Mk2",
    description: "Reduces warp fuel cost by 30%.",
    tier: 2,
    cost: 600,
  },
};

// -------------------------------------------------------------------------
// EconomyManager
// -------------------------------------------------------------------------

export class EconomyManager {
  constructor(private readonly gameState: GameState) {}

  /** Get the full economy state. */
  getState(): EconomyState {
    return this.gameState.getEconomy();
  }

  /** Mine an artifact for crystals. Returns crystals earned. */
  mineArtifact(rarity: RarityTier): number {
    const base = RARITY_YIELD[rarity];
    // Apply scanner bonus
    const hasScanner3 = this.hasUpgrade("scanner_mk3");
    const bonus = hasScanner3 ? 1.5 : 1.0;
    const earned = Math.round(base * bonus);
    this.gameState.addCrystals(earned);
    return earned;
  }

  /** Calculate warp fuel cost based on distance. */
  calculateWarpCost(distance: number): number {
    let cost = WARP_BASE_COST + distance * WARP_DISTANCE_FACTOR;
    if (this.hasUpgrade("warp_drive_mk2")) {
      cost *= 0.7;
    }
    return Math.ceil(cost);
  }

  /** Attempt to consume fuel for a warp. */
  tryWarp(distance: number): { success: boolean; cost: number } {
    const cost = this.calculateWarpCost(distance);
    const success = this.gameState.consumeFuel(cost);
    return { success, cost };
  }

  /** Get available upgrades (not yet purchased). */
  getAvailableUpgrades(): Array<ShipUpgrade & { cost: number }> {
    const owned = new Set(this.gameState.getShip().upgrades.map((u) => u.id));
    return Object.values(UPGRADE_CATALOG).filter((u) => !owned.has(u.id));
  }

  /** Get the catalog entry for an upgrade. */
  getUpgradeInfo(upgradeId: string): (ShipUpgrade & { cost: number }) | undefined {
    return UPGRADE_CATALOG[upgradeId];
  }

  /** Purchase an upgrade. Returns false if already owned or insufficient crystals. */
  purchaseUpgrade(upgradeId: string): boolean {
    const info = UPGRADE_CATALOG[upgradeId];
    if (!info) return false;

    const ship = this.gameState.getShip();
    if (ship.upgrades.find((u) => u.id === upgradeId)) return false;

    if (!this.gameState.spendCrystals(info.cost)) return false;

    ship.upgrades.push({
      id: info.id,
      name: info.name,
      description: info.description,
      tier: info.tier,
    });

    // Apply upgrade effects
    this.applyUpgradeEffect(upgradeId);

    return true;
  }

  /** Passive block staking reward (called on new block events). */
  onBlockReward(stakedAmount: number): number {
    if (stakedAmount <= 0) return 0;
    // 0.001 crystals per staked token per block
    const reward = Math.floor(stakedAmount * 0.001);
    if (reward > 0) {
      this.gameState.addCrystals(reward);
    }
    return reward;
  }

  /** Refuel ship (costs crystals). Returns fuel added. */
  refuel(crystalCost: number): number {
    if (!this.gameState.spendCrystals(crystalCost)) return 0;
    // 1 crystal = 2 fuel
    const fuelAdded = crystalCost * 2;
    const ship = this.gameState.getShip();
    const maxFuel = this.getMaxFuel();
    const actualAdded = Math.min(fuelAdded, maxFuel - ship.fuel);
    ship.fuel = Math.min(ship.fuel + fuelAdded, maxFuel);
    return actualAdded;
  }

  /** Get max fuel based on upgrades. */
  getMaxFuel(): number {
    if (this.hasUpgrade("fuel_tank_mk3")) return 300;
    if (this.hasUpgrade("fuel_tank_mk2")) return 200;
    return 100;
  }

  /** Get max health based on upgrades. */
  getMaxHealth(): number {
    return this.hasUpgrade("hull_reinforced") ? 150 : 100;
  }

  // -----------------------------------------------------------------------
  // Private
  // -----------------------------------------------------------------------

  private hasUpgrade(id: string): boolean {
    return this.gameState.getShip().upgrades.some((u) => u.id === id);
  }

  private applyUpgradeEffect(id: string): void {
    const ship = this.gameState.getShip();
    switch (id) {
      case "fuel_tank_mk2":
      case "fuel_tank_mk3":
        // Max fuel is computed dynamically via getMaxFuel()
        break;
      case "hull_reinforced":
        ship.health = Math.min(ship.health + 50, this.getMaxHealth());
        break;
      default:
        break;
    }
  }
}
