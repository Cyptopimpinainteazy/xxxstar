/**
 * @module scene/EntityFactory
 * Factory that creates Three.js meshes for every SceneEntityKind.
 *
 * Each entity has a distinct visual identity:
 * - blockchain_node: glowing icosahedron
 * - data_flow: animated tube geometry
 * - contract: crystalline dodecahedron
 * - vault: shielded sphere with hex pattern
 * - planet: procedural textured sphere
 * - voyager_ship: low-poly spaceship group
 * - artifact: rarity-tinted gem
 * - waypoint: beacon pillar
 */
import * as THREE from "three";
import type { SceneEntityKind } from "../types/scene";
import type { RarityTier } from "../types/game";

const CYAN = 0x00f0ff;
const MAGENTA = 0xff00ff;
const GOLD = 0xffd700;
const GREEN = 0x00ff88;
const WHITE = 0xffffff;

const RARITY_COLORS: Record<RarityTier, number> = {
  Common: 0x888888,
  Uncommon: GREEN,
  Rare: CYAN,
  Epic: MAGENTA,
  Legendary: GOLD,
};

/** Create a complete Three.js Object3D for a given entity kind. */
export function createEntityMesh(
  kind: SceneEntityKind,
  options: EntityOptions = {},
): THREE.Object3D {
  switch (kind) {
    case "blockchain_node": return createBlockchainNode(options);
    case "data_flow": return createDataFlow(options);
    case "contract": return createContract(options);
    case "vault": return createVault(options);
    case "planet": return createPlanet(options);
    case "voyager_ship": return createVoyagerShip(options);
    case "artifact": return createArtifact(options);
    case "waypoint": return createWaypoint(options);
    default: return createFallback();
  }
}

export interface EntityOptions {
  color?: number;
  scale?: number;
  rarity?: RarityTier;
  radius?: number;
  seed?: number;
}

// -------------------------------------------------------------------------
// blockchain_node — glowing icosahedron
// -------------------------------------------------------------------------

function createBlockchainNode(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const radius = opts.radius ?? 1.5;
  const color = opts.color ?? CYAN;

  const geo = new THREE.IcosahedronGeometry(radius, 1);
  const mat = new THREE.MeshStandardMaterial({
    color,
    emissive: color,
    emissiveIntensity: 0.6,
    metalness: 0.9,
    roughness: 0.1,
    wireframe: false,
  });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.castShadow = true;
  group.add(mesh);

  // Wireframe overlay
  const wireMat = new THREE.MeshBasicMaterial({ color, wireframe: true, transparent: true, opacity: 0.3 });
  const wire = new THREE.Mesh(new THREE.IcosahedronGeometry(radius * 1.05, 1), wireMat);
  group.add(wire);

  // Inner glow point light
  const light = new THREE.PointLight(color, 1.5, radius * 8);
  group.add(light);

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// data_flow — animated tube
// -------------------------------------------------------------------------

function createDataFlow(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const color = opts.color ?? CYAN;

  // Create a curved path
  const curve = new THREE.CatmullRomCurve3([
    new THREE.Vector3(-5, 0, 0),
    new THREE.Vector3(-2, 3, 2),
    new THREE.Vector3(2, -1, -1),
    new THREE.Vector3(5, 1, 0),
  ]);

  const tubeGeo = new THREE.TubeGeometry(curve, 64, 0.08, 8, false);
  const tubeMat = new THREE.MeshStandardMaterial({
    color,
    emissive: color,
    emissiveIntensity: 0.8,
    transparent: true,
    opacity: 0.7,
  });
  group.add(new THREE.Mesh(tubeGeo, tubeMat));

  // Data particles along the curve (instanced spheres)
  const particleCount = 8;
  const particleGeo = new THREE.SphereGeometry(0.12, 8, 8);
  const particleMat = new THREE.MeshBasicMaterial({ color: WHITE });
  const instanced = new THREE.InstancedMesh(particleGeo, particleMat, particleCount);
  const dummy = new THREE.Object3D();
  for (let i = 0; i < particleCount; i++) {
    const t = i / particleCount;
    const p = curve.getPointAt(t);
    dummy.position.copy(p);
    dummy.updateMatrix();
    instanced.setMatrixAt(i, dummy.matrix);
  }
  instanced.instanceMatrix.needsUpdate = true;
  instanced.userData.curve = curve;
  instanced.userData.particleCount = particleCount;
  group.add(instanced);

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// contract — crystalline dodecahedron
// -------------------------------------------------------------------------

function createContract(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const color = opts.color ?? MAGENTA;

  const geo = new THREE.DodecahedronGeometry(1.2, 0);
  const mat = new THREE.MeshPhysicalMaterial({
    color,
    emissive: color,
    emissiveIntensity: 0.3,
    metalness: 0.2,
    roughness: 0.05,
    transmission: 0.6,
    thickness: 0.5,
    transparent: true,
    opacity: 0.85,
  });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.castShadow = true;
  group.add(mesh);

  // Edges
  const edges = new THREE.EdgesGeometry(geo);
  const line = new THREE.LineSegments(edges, new THREE.LineBasicMaterial({ color, linewidth: 1 }));
  group.add(line);

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// vault — shielded sphere
// -------------------------------------------------------------------------

function createVault(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const radius = opts.radius ?? 2;
  const color = opts.color ?? GOLD;

  // Inner core
  const coreGeo = new THREE.SphereGeometry(radius * 0.6, 32, 32);
  const coreMat = new THREE.MeshStandardMaterial({
    color,
    emissive: color,
    emissiveIntensity: 0.5,
    metalness: 1.0,
    roughness: 0.2,
  });
  const core = new THREE.Mesh(coreGeo, coreMat);
  core.castShadow = true;
  group.add(core);

  // Shield shell (wireframe icosahedron)
  const shieldGeo = new THREE.IcosahedronGeometry(radius, 2);
  const shieldMat = new THREE.MeshBasicMaterial({
    color: CYAN,
    wireframe: true,
    transparent: true,
    opacity: 0.25,
  });
  group.add(new THREE.Mesh(shieldGeo, shieldMat));

  // Orbiting ring
  const ringGeo = new THREE.TorusGeometry(radius * 1.2, 0.05, 16, 64);
  const ringMat = new THREE.MeshBasicMaterial({ color: CYAN, transparent: true, opacity: 0.5 });
  const ring = new THREE.Mesh(ringGeo, ringMat);
  ring.rotation.x = Math.PI / 2;
  group.add(ring);

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// planet — procedural sphere
// -------------------------------------------------------------------------

function createPlanet(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const radius = opts.radius ?? 8;
  const color = opts.color ?? 0x2288aa;

  const geo = new THREE.SphereGeometry(radius, 64, 64);
  // Displace vertices for terrain
  const seed = opts.seed ?? 42;
  const posAttr = geo.getAttribute("position");
  for (let i = 0; i < posAttr.count; i++) {
    const x = posAttr.getX(i);
    const y = posAttr.getY(i);
    const z = posAttr.getZ(i);
    const len = Math.sqrt(x * x + y * y + z * z);
    const nx = x / len;
    const ny = y / len;
    const nz = z / len;
    // Pseudo-noise displacement from seed
    const noise = pseudoNoise3D(nx * 3 + seed, ny * 3, nz * 3);
    const displacement = 1 + noise * 0.08;
    posAttr.setXYZ(i, x * displacement, y * displacement, z * displacement);
  }
  geo.computeVertexNormals();

  const mat = new THREE.MeshStandardMaterial({
    color,
    metalness: 0.1,
    roughness: 0.8,
    flatShading: true,
  });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.castShadow = true;
  mesh.receiveShadow = true;
  group.add(mesh);

  // Atmosphere halo
  const atmosGeo = new THREE.SphereGeometry(radius * 1.05, 32, 32);
  const atmosMat = new THREE.MeshBasicMaterial({
    color: 0x88ccff,
    transparent: true,
    opacity: 0.15,
    side: THREE.BackSide,
  });
  group.add(new THREE.Mesh(atmosGeo, atmosMat));

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// voyager_ship — low-poly spaceship
// -------------------------------------------------------------------------

function createVoyagerShip(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const color = opts.color ?? WHITE;

  // Fuselage — elongated octahedron
  const bodyGeo = new THREE.CylinderGeometry(0.3, 0.8, 3, 6);
  const bodyMat = new THREE.MeshStandardMaterial({
    color,
    metalness: 0.8,
    roughness: 0.2,
  });
  const body = new THREE.Mesh(bodyGeo, bodyMat);
  body.rotation.x = Math.PI / 2;
  body.castShadow = true;
  group.add(body);

  // Wings
  const wingGeo = new THREE.BoxGeometry(4, 0.05, 1);
  const wingMat = new THREE.MeshStandardMaterial({ color: 0xcccccc, metalness: 0.7, roughness: 0.3 });
  const wings = new THREE.Mesh(wingGeo, wingMat);
  wings.position.z = 0.3;
  wings.castShadow = true;
  group.add(wings);

  // Engine glow
  const engineGeo = new THREE.SphereGeometry(0.25, 8, 8);
  const engineMat = new THREE.MeshBasicMaterial({ color: CYAN });
  const engineL = new THREE.Mesh(engineGeo, engineMat);
  engineL.position.set(-0.6, 0, 1.4);
  group.add(engineL);

  const engineR = new THREE.Mesh(engineGeo, engineMat);
  engineR.position.set(0.6, 0, 1.4);
  group.add(engineR);

  // Engine point lights
  const engineLightL = new THREE.PointLight(CYAN, 2, 5);
  engineLightL.position.copy(engineL.position);
  group.add(engineLightL);

  const engineLightR = new THREE.PointLight(CYAN, 2, 5);
  engineLightR.position.copy(engineR.position);
  group.add(engineLightR);

  // Cockpit
  const cockpitGeo = new THREE.SphereGeometry(0.35, 8, 8, 0, Math.PI * 2, 0, Math.PI / 2);
  const cockpitMat = new THREE.MeshPhysicalMaterial({
    color: 0x224488,
    metalness: 0.1,
    roughness: 0.05,
    transmission: 0.8,
    thickness: 0.3,
    transparent: true,
  });
  const cockpit = new THREE.Mesh(cockpitGeo, cockpitMat);
  cockpit.position.z = -1.2;
  cockpit.rotation.x = -Math.PI / 2;
  group.add(cockpit);

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// artifact — rarity-tinted gem
// -------------------------------------------------------------------------

function createArtifact(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const rarity = opts.rarity ?? "Common";
  const color = RARITY_COLORS[rarity];

  // Octahedron gem
  const geo = new THREE.OctahedronGeometry(0.8, 0);
  const mat = new THREE.MeshPhysicalMaterial({
    color,
    emissive: color,
    emissiveIntensity: rarity === "Legendary" ? 1.0 : 0.4,
    metalness: 0.3,
    roughness: 0.0,
    transmission: 0.4,
    thickness: 0.5,
    transparent: true,
  });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.castShadow = true;
  group.add(mesh);

  // Floating particles ring for rare+
  if (rarity !== "Common") {
    const ringGeo = new THREE.TorusGeometry(1.2, 0.02, 8, 32);
    const ringMat = new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.4 });
    const ring = new THREE.Mesh(ringGeo, ringMat);
    ring.rotation.x = Math.PI / 3;
    group.add(ring);
  }

  // Point light for epic+
  if (rarity === "Epic" || rarity === "Legendary") {
    const light = new THREE.PointLight(color, 3, 8);
    group.add(light);
  }

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// waypoint — beacon pillar
// -------------------------------------------------------------------------

function createWaypoint(opts: EntityOptions): THREE.Group {
  const group = new THREE.Group();
  const color = opts.color ?? GREEN;

  // Pillar
  const pillarGeo = new THREE.CylinderGeometry(0.15, 0.15, 4, 8);
  const pillarMat = new THREE.MeshStandardMaterial({
    color: 0x444444,
    metalness: 0.6,
    roughness: 0.4,
  });
  const pillar = new THREE.Mesh(pillarGeo, pillarMat);
  pillar.position.y = 2;
  pillar.castShadow = true;
  group.add(pillar);

  // Beacon orb on top
  const orbGeo = new THREE.SphereGeometry(0.3, 16, 16);
  const orbMat = new THREE.MeshBasicMaterial({ color });
  const orb = new THREE.Mesh(orbGeo, orbMat);
  orb.position.y = 4.3;
  group.add(orb);

  const light = new THREE.PointLight(color, 2, 15);
  light.position.y = 4.3;
  group.add(light);

  // Base ring
  const baseGeo = new THREE.TorusGeometry(0.6, 0.08, 8, 24);
  const baseMat = new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.5 });
  const base = new THREE.Mesh(baseGeo, baseMat);
  base.rotation.x = Math.PI / 2;
  group.add(base);

  if (opts.scale) group.scale.setScalar(opts.scale);
  return group;
}

// -------------------------------------------------------------------------
// Fallback — simple box
// -------------------------------------------------------------------------

function createFallback(): THREE.Mesh {
  return new THREE.Mesh(
    new THREE.BoxGeometry(1, 1, 1),
    new THREE.MeshStandardMaterial({ color: 0xff0000 }),
  );
}

// -------------------------------------------------------------------------
// Utility
// -------------------------------------------------------------------------

/** Cheap deterministic 3D pseudo-noise (not simplex, just for vertex displacement). */
function pseudoNoise3D(x: number, y: number, z: number): number {
  const n = Math.sin(x * 12.9898 + y * 78.233 + z * 37.719) * 43758.5453;
  return (n - Math.floor(n)) * 2 - 1;
}
