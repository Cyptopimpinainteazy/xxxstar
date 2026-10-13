/**
 * @module game/WorldManager
 * Manages world discovery, loading, and lifecycle.
 *
 * Coordinates ProceduralGenerator → EntityFactory → SceneManager to
 * spawn a fully populated 3D world from a seed string.
 */
import type { DiscoveredWorld, WorldParams, ArtifactRecord, Vec3 } from "../types/game";
import type { SceneEntity, BlockchainNodeEntity } from "../types/scene";
import { generateWorld, generateArtifacts, generateHeightmap } from "../scene/ProceduralGenerator";
import { createEntityMesh } from "../scene/EntityFactory";
import type { SceneManager } from "../scene/SceneManager";
// three is used indirectly via EntityFactory

const WORLD_RADIUS = 50;
const ARTIFACT_COUNT = 12;
const NODE_COUNT = 8;

export class WorldManager {
  private currentSeed: string | null = null;
  private currentParams: WorldParams | null = null;
  private spawnedEntityIds: string[] = [];

  constructor(private readonly sceneManager: SceneManager) {}

  /** Currently loaded world seed, or null. */
  getCurrentSeed(): string | null {
    return this.currentSeed;
  }

  getCurrentParams(): WorldParams | null {
    return this.currentParams;
  }

  /**
   * Load a world by seed. Generates terrain, artifacts, blockchain nodes.
   * Unloads any previously loaded world first.
   */
  loadWorld(seed: string): { world: DiscoveredWorld; params: WorldParams; artifacts: ArtifactRecord[] } {
    this.unloadWorld();

    const { world, params } = generateWorld(seed);
    this.currentSeed = seed;
    this.currentParams = params;

    // Generate heightmap for later use (e.g., minimap)
    // Generate heightmap (available for minimap)
    void generateHeightmap(params, 64);

    // Spawn planet
    const planetId = `planet-${seed}`;
    const planetMesh = createEntityMesh("planet", {
      radius: WORLD_RADIUS,
      seed: hashSeed(seed),
      color: biomeColor(params.biome),
    });
    const planetEntity: SceneEntity = {
      id: planetId,
      kind: "planet",
      position: { x: 0, y: 0, z: 0 },
      label: params.name,
    };
    this.sceneManager.addEntity(planetEntity, planetMesh);
    this.spawnedEntityIds.push(planetId);

    // Spawn artifacts on surface
    const artifacts = generateArtifacts(seed, WORLD_RADIUS, ARTIFACT_COUNT);
    for (const artifact of artifacts) {
      const artId = artifact.id;
      const mesh = createEntityMesh("artifact", { rarity: artifact.rarity });
      mesh.position.set(artifact.position.x, artifact.position.y, artifact.position.z);
      const entity: SceneEntity = {
        id: artId,
        kind: "artifact",
        position: artifact.position,
        label: artifact.name,
      };
      this.sceneManager.addEntity(entity, mesh);
      this.spawnedEntityIds.push(artId);
    }

    // Spawn blockchain nodes in orbit
    for (let i = 0; i < NODE_COUNT; i++) {
      const angle = (i / NODE_COUNT) * Math.PI * 2;
      const orbitR = WORLD_RADIUS * 1.8;
      const pos: Vec3 = {
        x: Math.cos(angle) * orbitR,
        y: (Math.random() - 0.5) * 20,
        z: Math.sin(angle) * orbitR,
      };
      const nodeId = `node-${seed}-${i}`;
      const mesh = createEntityMesh("blockchain_node", { radius: 2 });
      mesh.position.set(pos.x, pos.y, pos.z);
      const nodeEntity: BlockchainNodeEntity = {
        id: nodeId,
        kind: "blockchain_node",
        position: pos,
        label: `Node ${i}`,
        blockHeight: 0,
        metadata: { chainId: "atlas-sphere-devnet", peerCount: Math.floor(Math.random() * 10) },
      };
      this.sceneManager.addEntity(nodeEntity, mesh);
      this.spawnedEntityIds.push(nodeId);
    }

    // Spawn waypoints at cardinal directions
    const directions = [
      { x: WORLD_RADIUS * 2.5, y: 0, z: 0 },
      { x: -WORLD_RADIUS * 2.5, y: 0, z: 0 },
      { x: 0, y: 0, z: WORLD_RADIUS * 2.5 },
      { x: 0, y: 0, z: -WORLD_RADIUS * 2.5 },
    ];
    for (let i = 0; i < directions.length; i++) {
      const wpId = `waypoint-${seed}-${i}`;
      const mesh = createEntityMesh("waypoint");
      mesh.position.set(directions[i].x, directions[i].y, directions[i].z);
      const entity: SceneEntity = {
        id: wpId,
        kind: "waypoint",
        position: directions[i],
        label: `Waypoint ${["N", "S", "E", "W"][i]}`,
      };
      this.sceneManager.addEntity(entity, mesh);
      this.spawnedEntityIds.push(wpId);
    }

    // Mark world visited
    world.visited = true;

    return { world, params, artifacts };
  }

  /** Remove all entities spawned for the current world. */
  unloadWorld(): void {
    for (const id of this.spawnedEntityIds) {
      this.sceneManager.removeEntity(id);
    }
    this.spawnedEntityIds = [];
    this.currentSeed = null;
    this.currentParams = null;
  }

  /** Update orbiting node block heights (call from chain event). */
  updateNodeBlockHeight(chainId: string, height: number): void {
    for (const id of this.spawnedEntityIds) {
      if (!id.startsWith("node-")) continue;
      const entity = this.sceneManager.getEntity(id) as BlockchainNodeEntity | undefined;
      if (entity && entity.metadata && (entity.metadata as Record<string, unknown>).chainId === chainId) {
        (entity as { blockHeight: number }).blockHeight = height;
      }
    }
  }
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

function hashSeed(seed: string): number {
  let h = 0;
  for (let i = 0; i < seed.length; i++) {
    h = (Math.imul(31, h) + seed.charCodeAt(i)) | 0;
  }
  return h;
}

function biomeColor(biome: string): number {
  const map: Record<string, number> = {
    "Crystal Fields": 0x88ccff,
    "Data Swamp": 0x336633,
    "Quantum Desert": 0xddaa44,
    "Neon Jungle": 0x00ff88,
    "Void Tundra": 0xaabbcc,
    "Plasma Ocean": 0x4488ff,
    "Circuit Mountains": 0x887766,
    "Ether Plains": 0x66aa88,
  };
  return map[biome] ?? 0x888888;
}
