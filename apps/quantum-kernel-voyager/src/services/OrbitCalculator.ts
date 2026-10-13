/**
 * @module services/OrbitCalculator
 * Computes orbit radii, planet sizes, and orbital speeds from value data.
 *
 * Higher market cap → closer to sun (smaller orbit radius).
 * Uses logarithmic scaling for both orbit and size to prevent extremes.
 * Orbital speed follows Kepler: inner planets orbit faster.
 */
import type { PlanetEntity } from "../types/solar-system";
import {
  MIN_ORBIT_RADIUS,
  MAX_ORBIT_RADIUS,
  MIN_PLANET_RADIUS,
  MAX_PLANET_RADIUS,
  BASE_ORBIT_SPEED,
} from "../config/solar-system-config";

/**
 * Compute orbit radius from a planet's value ranking.
 * Rank 1 (highest value) → MIN_ORBIT_RADIUS.
 * Lowest rank → MAX_ORBIT_RADIUS.
 *
 * @example
 * ```ts
 * computeOrbitRadius(1, 10, 30, 200); // ~30 (closest)
 * computeOrbitRadius(10, 10, 30, 200); // ~200 (farthest)
 * ```
 */
export function computeOrbitRadius(
  rank: number,
  totalPlanets: number,
  minRadius = MIN_ORBIT_RADIUS,
  maxRadius = MAX_ORBIT_RADIUS,
): number {
  if (totalPlanets <= 1) return (minRadius + maxRadius) / 2;
  // Logarithmic spread so top-ranked planets have more separation
  const t = (rank - 1) / (totalPlanets - 1); // 0..1
  const logT = Math.log1p(t * 9) / Math.log(10); // log10(1 + t*9) → 0..1
  return minRadius + logT * (maxRadius - minRadius);
}

/**
 * Compute planet visual radius from market cap.
 * Uses cube-root scaling to prevent extreme size differences.
 *
 * @example
 * ```ts
 * computePlanetRadius(500_000_000, 500_000_000, 1.5, 8); // 8 (max)
 * computePlanetRadius(0, 500_000_000, 1.5, 8); // 1.5 (min)
 * ```
 */
export function computePlanetRadius(
  marketCap: number,
  maxMarketCap: number,
  minSize = MIN_PLANET_RADIUS,
  maxSize = MAX_PLANET_RADIUS,
): number {
  if (maxMarketCap <= 0) return minSize;
  const ratio = Math.max(0, Math.min(1, marketCap / maxMarketCap));
  const cubeRoot = Math.cbrt(ratio);
  return minSize + cubeRoot * (maxSize - minSize);
}

/**
 * Compute orbital speed for a given orbit radius.
 * Inner orbits are faster (Keplerian approximation: v ∝ r^-0.5).
 */
export function computeOrbitSpeed(orbitRadius: number): number {
  return BASE_ORBIT_SPEED * Math.sqrt(MIN_ORBIT_RADIUS / orbitRadius);
}

/**
 * Recalculate orbit radii, sizes, and speeds for all planets.
 * Planets are ranked by marketCap (highest = rank 1 = closest to sun).
 * Mutates the planet objects in place.
 */
export function recalculateOrbits(planets: PlanetEntity[]): void {
  if (planets.length === 0) return;

  // Sort by market cap descending for ranking
  const ranked = [...planets].sort((a, b) => b.marketCap - a.marketCap);
  const maxCap = ranked[0].marketCap;
  const total = ranked.length;

  for (let i = 0; i < total; i++) {
    const p = ranked[i];
    const rank = i + 1;

    p.orbitRadius = computeOrbitRadius(rank, total);
    p.planetRadius = computePlanetRadius(p.marketCap, maxCap);
    p.orbitSpeed = computeOrbitSpeed(p.orbitRadius);
  }
}

/**
 * Advance all planets along their orbits by dt seconds.
 */
export function advanceOrbits(planets: PlanetEntity[], dt: number, timeScale: number): void {
  for (const p of planets) {
    p.orbitAngle += p.orbitSpeed * dt * timeScale;
    // Keep angle normalized
    if (p.orbitAngle > Math.PI * 2) p.orbitAngle -= Math.PI * 2;
  }
}

/**
 * Get a planet's 3D world position from its orbit parameters.
 */
export function getPlanetPosition(planet: PlanetEntity): { x: number; y: number; z: number } {
  const { orbitRadius, orbitAngle, orbitInclination } = planet;
  return {
    x: Math.cos(orbitAngle) * orbitRadius,
    y: Math.sin(orbitInclination) * Math.sin(orbitAngle) * orbitRadius * 0.3,
    z: Math.sin(orbitAngle) * orbitRadius,
  };
}
