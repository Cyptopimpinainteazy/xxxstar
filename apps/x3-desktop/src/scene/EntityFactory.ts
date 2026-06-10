import * as THREE from 'three';

/**
 * Factory for creating agent geometries.
 * Uses a shared geometry cache for performance.
 */
const geometryCache = new Map<string, THREE.BufferGeometry>();

export function EntityFactory({ type }: { type: AgentEntityType }): THREE.BufferGeometry {
  let geometry = geometryCache.get(type);
  if (!geometry) {
    geometry = createGeometry(type);
    geometryCache.set(type, geometry);
  }
  return geometry;
}

export type AgentEntityType = 'sphere' | 'cube' | 'diamond' | 'cylinder' | 'torus' | 'cone' | 'icosahedron';

function createGeometry(type: AgentEntityType): THREE.BufferGeometry {
  switch (type) {
    case 'sphere':
      return new THREE.SphereGeometry(0.8, 24, 24);
    case 'cube':
      return new THREE.BoxGeometry(1.0, 1.0, 1.0);
    case 'diamond':
      return new THREE.OctahedronGeometry(0.9);
    case 'cylinder':
      return new THREE.CylinderGeometry(0.6, 0.6, 1.2, 16);
    case 'torus':
      return new THREE.TorusGeometry(0.6, 0.25, 16, 24);
    case 'cone':
      return new THREE.ConeGeometry(0.7, 1.2, 16);
    case 'icosahedron':
      return new THREE.IcosahedronGeometry(0.8);
    default:
      return new THREE.SphereGeometry(0.8, 24, 24);
  }
}

EntityFactory.displayName = 'EntityFactory';