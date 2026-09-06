/**
 * Factory for creating agent geometries as React Three Fiber components.
 */
export type AgentEntityType = 'sphere' | 'cube' | 'diamond' | 'cylinder' | 'torus' | 'cone' | 'icosahedron';

export function EntityFactory({ type }: { type: AgentEntityType }): JSX.Element {
  switch (type) {
    case 'sphere':
      return <sphereGeometry args={[0.8, 24, 24]} />;
    case 'cube':
      return <boxGeometry args={[1.0, 1.0, 1.0]} />;
    case 'diamond':
      return <octahedronGeometry args={[0.9]} />;
    case 'cylinder':
      return <cylinderGeometry args={[0.6, 0.6, 1.2, 16]} />;
    case 'torus':
      return <torusGeometry args={[0.6, 0.25, 16, 24]} />;
    case 'cone':
      return <coneGeometry args={[0.7, 1.2, 16]} />;
    case 'icosahedron':
      return <icosahedronGeometry args={[0.8]} />;
    default:
      return <sphereGeometry args={[0.8, 24, 24]} />;
  }
}

EntityFactory.displayName = 'EntityFactory';