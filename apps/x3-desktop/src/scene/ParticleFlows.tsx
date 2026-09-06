import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

interface ParticleFlowsProps {
  count: number;
  color: string;
  spread?: number;
}

/**
 * Instanced particle system for ambient arena activity.
 * Renders as floating particles that drift upward like data flow.
 */
export function ParticleFlows({ count, color, spread = 20 }: ParticleFlowsProps) {
  const meshRef = useRef<THREE.InstancedMesh>(null!);
  const dummy = useMemo(() => new THREE.Object3D(), []);

  // Per-particle velocities and phases
  const particles = useMemo(() => {
    const data: { x: number; y: number; z: number; speed: number; phase: number }[] = [];
    for (let i = 0; i < count; i++) {
      data.push({
        x: (Math.random() - 0.5) * spread,
        y: (Math.random() - 0.5) * spread,
        z: (Math.random() - 0.5) * spread,
        speed: 0.2 + Math.random() * 0.5,
        phase: Math.random() * Math.PI * 2,
      });
    }
    return data;
  }, [count, spread]);

  useFrame((state) => {
    if (!meshRef.current) return;
    const t = state.clock.elapsedTime;

    for (let i = 0; i < count; i++) {
      const p = particles[i];
      // Drift upward with gentle horizontal oscillation
      const x = p.x + Math.sin(t * 0.3 + p.phase) * 1.5;
      const y = ((p.y + t * p.speed * 0.8) % spread) - spread / 2;
      const z = p.z + Math.cos(t * 0.2 + p.phase) * 1.5;

      dummy.position.set(x, y, z);
      const scale = 0.05 + Math.sin(t + p.phase) * 0.02;
      dummy.scale.setScalar(scale);
      dummy.updateMatrix();
      meshRef.current.setMatrixAt(i, dummy.matrix);
    }
    meshRef.current.instanceMatrix.needsUpdate = true;
  });

  return (
    <instancedMesh
      ref={meshRef}
      args={[undefined, undefined, count]}
      frustumCulled={false}
    >
      <sphereGeometry args={[1, 6, 6]} />
      <meshBasicMaterial color={color} transparent opacity={0.3} />
    </instancedMesh>
  );
}