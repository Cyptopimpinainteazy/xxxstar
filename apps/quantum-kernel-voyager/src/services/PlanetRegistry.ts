/**
 * @module services/PlanetRegistry
 * Central registry for all planets in the solar system.
 *
 * Manages adding/removing planets, value updates, and proximity queries.
 * Initialized from CORE_PLANETS, then augmented with on-chain data.
 */
import type { PlanetEntity, SwarmNodeInfo, FactoryInfo, SolarNotification } from "../types/solar-system";
import { CORE_PLANETS } from "../config/solar-system-config";
import { recalculateOrbits, getPlanetPosition } from "./OrbitCalculator";

export class PlanetRegistry {
  private planets: Map<string, PlanetEntity> = new Map();
  private notifications: SolarNotification[] = [];

  constructor() {
    // Load core ecosystem planets
    for (const p of CORE_PLANETS) {
      this.planets.set(p.id, { ...p });
    }
    recalculateOrbits(this.getAll());
  }

  /** Get all planets as an array. */
  getAll(): PlanetEntity[] {
    return Array.from(this.planets.values());
  }

  /** Get planet by ID. */
  get(id: string): PlanetEntity | undefined {
    return this.planets.get(id);
  }

  /** Add a new planet (e.g., from on-chain token discovery). */
  addPlanet(planet: PlanetEntity): void {
    this.planets.set(planet.id, planet);
    recalculateOrbits(this.getAll());
    this.notify(`New planet discovered: ${planet.name}`, "info");
  }

  /** Remove a planet. */
  removePlanet(id: string): void {
    this.planets.delete(id);
    recalculateOrbits(this.getAll());
  }

  /** Update market data for a planet. Triggers orbit recalculation. */
  updateMarketData(id: string, data: { marketCap?: number; price?: number; volume24h?: number; priceChange24h?: number }): void {
    const p = this.planets.get(id);
    if (!p) return;
    if (data.marketCap !== undefined) p.marketCap = data.marketCap;
    if (data.price !== undefined) p.price = data.price;
    if (data.volume24h !== undefined) p.volume24h = data.volume24h;
    if (data.priceChange24h !== undefined) p.priceChange24h = data.priceChange24h;
    recalculateOrbits(this.getAll());
  }

  /** Assign a swarm node to a planet. */
  addSwarmNode(planetId: string, node: SwarmNodeInfo): void {
    const p = this.planets.get(planetId);
    if (!p) return;
    // Replace if same nodeId, else add
    const idx = p.swarmNodes.findIndex((n) => n.nodeId === node.nodeId);
    if (idx >= 0) {
      p.swarmNodes[idx] = node;
    } else {
      p.swarmNodes.push(node);
    }
  }

  /** Remove a swarm node. */
  removeSwarmNode(planetId: string, nodeId: string): void {
    const p = this.planets.get(planetId);
    if (!p) return;
    p.swarmNodes = p.swarmNodes.filter((n) => n.nodeId !== nodeId);
  }

  /** Add a factory to a planet. */
  addFactory(planetId: string, factory: FactoryInfo): void {
    const p = this.planets.get(planetId);
    if (!p) return;
    p.factories.push(factory);
    this.notify(`New dApp on ${p.name}: ${factory.name}`, "success");
  }

  /**
   * Find planets within a given distance of a world position.
   * Used for landing proximity detection.
   */
  findNearby(position: { x: number; y: number; z: number }, maxDistance: number): PlanetEntity[] {
    const results: PlanetEntity[] = [];
    for (const p of this.planets.values()) {
      const pp = getPlanetPosition(p);
      const dx = pp.x - position.x;
      const dy = pp.y - position.y;
      const dz = pp.z - position.z;
      const dist = Math.sqrt(dx * dx + dy * dy + dz * dz);
      if (dist < maxDistance) {
        results.push(p);
      }
    }
    return results;
  }

  /**
   * Find the single closest planet within landing range.
   */
  findLandingCandidate(position: { x: number; y: number; z: number }): PlanetEntity | null {
    let closest: PlanetEntity | null = null;
    let closestDist = Infinity;

    for (const p of this.planets.values()) {
      const pp = getPlanetPosition(p);
      const dx = pp.x - position.x;
      const dy = pp.y - position.y;
      const dz = pp.z - position.z;
      const dist = Math.sqrt(dx * dx + dy * dy + dz * dz);
      const threshold = p.planetRadius * 3.5; // LANDING_PROXIMITY_MULTIPLIER
      if (dist < threshold && dist < closestDist) {
        closest = p;
        closestDist = dist;
      }
    }
    return closest;
  }

  // -----------------------------------------------------------------------
  // Notifications
  // -----------------------------------------------------------------------

  private notify(message: string, type: SolarNotification["type"]): void {
    this.notifications.push({
      id: `notif-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      message,
      type,
      timestamp: Date.now(),
      dismissed: false,
    });
    // Keep last 50
    if (this.notifications.length > 50) {
      this.notifications = this.notifications.slice(-50);
    }
  }

  getNotifications(): SolarNotification[] {
    return this.notifications.filter((n) => !n.dismissed);
  }

  dismissNotification(id: string): void {
    const n = this.notifications.find((x) => x.id === id);
    if (n) n.dismissed = true;
  }
}
