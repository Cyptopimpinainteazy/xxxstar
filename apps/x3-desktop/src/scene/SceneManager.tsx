import React, { useRef, useEffect, useCallback } from 'react';
import { Canvas, useFrame, useThree, ThreeEvent } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls, Text, Line, Html } from '@react-three/drei';
import { CameraController } from './CameraController';
import { EntityFactory } from './EntityFactory';
import { ParticleFlows } from './ParticleFlows';
import { useAgentStore, AgentState } from '../agents/AgentStore';
import { useBlockStore, BlockEntry } from '../blockchain/BlockStore';

interface SceneManagerProps {
  selectedAgentId: string | null;
  onAgentSelect: (id: string) => void;
  onAgentDeselect: () => void;
}

/* ─── Arena Grid Floor ─────────────────────────── */
function ArenaFloor() {
  return (
    <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -8, 0]} receiveShadow>
      <planeGeometry args={[40, 40]} />
      <meshStandardMaterial
        color="#0a1630"
        transparent
        opacity={0.6}
        roughness={0.8}
        metalness={0.2}
      />
    </mesh>
  );
}

/* ─── Arena Boundary Walls (glow wireframe) ─── */
function ArenaBoundary() {
  const size = 20;
  const height = 16;
  const edges = React.useMemo(() => {
    const g = new THREE.BoxGeometry(size, height, size);
    return new THREE.EdgesGeometry(g);
  }, [size, height]);

  return (
    <lineSegments geometry={edges} position={[0, 0, 0]}>
      <lineBasicMaterial color="#00aaff" transparent opacity={0.25} />
    </lineSegments>
  );
}

/* ─── Agent Entity 3D ──────────────────────────── */
function AgentMesh({ agent, isSelected, onClick }: {
  agent: AgentState;
  isSelected: boolean;
  onClick: (id: string) => void;
}) {
  const meshRef = useRef<THREE.Mesh>(null!);
  const ringRef = useRef<THREE.Mesh>(null!);
  const targetPos = useRef(new THREE.Vector3(agent.position.x, agent.position.y, agent.position.z));

  // Smooth movement toward target
  useFrame((_, delta) => {
    if (!meshRef.current || !ringRef.current) return;
    const speed = 2.0 * delta;
    meshRef.current.position.lerp(targetPos.current, speed);
    ringRef.current.position.copy(meshRef.current.position);
    ringRef.current.rotation.x += delta * 0.5;
    ringRef.current.rotation.y += delta * 0.8;

    // Pulsing scale when selected
    if (isSelected) {
      const pulse = 1 + Math.sin(Date.now() * 0.005) * 0.05;
      meshRef.current.scale.setScalar(pulse);
      ringRef.current.scale.setScalar(pulse);
    }
  });

  // Update target when agent position changes
  useEffect(() => {
    targetPos.current.set(agent.position.x, agent.position.y, agent.position.z);
  }, [agent.position.x, agent.position.y, agent.position.z]);

  const handleClick = useCallback((e: ThreeEvent<MouseEvent>) => {
    e.stopPropagation();
    onClick(agent.id);
  }, [agent.id, onClick]);

  const color = agent.color;
  const emissiveColor = isSelected ? '#ff8800' : color;

  return (
    <group>
      {/* Body */}
      <mesh
        ref={meshRef}
        onClick={handleClick}
        castShadow
      >
        <EntityFactory type={agent.entityType} />
        <meshStandardMaterial
          color={color}
          emissive={emissiveColor}
          emissiveIntensity={isSelected ? 0.6 : 0.15}
          roughness={0.3}
          metalness={0.7}
        />
      </mesh>

      {/* Selection ring */}
      <mesh ref={ringRef}>
        <ringGeometry args={[1.2, 1.5, 32]} />
        <meshBasicMaterial
          color={isSelected ? '#ff8800' : '#00aaff'}
          transparent
          opacity={isSelected ? 0.8 : 0.3}
          side={THREE.DoubleSide}
        />
      </mesh>

      {/* Health bar (billboard) */}
      <Html
        position={[agent.position.x, agent.position.y + 2.2, agent.position.z]}
        center
        style={{ pointerEvents: 'none' }}
      >
        <div style={{
          width: 60,
          height: 6,
          background: '#1a1a2e',
          borderRadius: 3,
          overflow: 'hidden',
          border: '1px solid #333'
        }}>
          <div style={{
            width: `${agent.health}%`,
            height: '100%',
            background: agent.health > 50 ? '#00cc66' : agent.health > 25 ? '#ffaa00' : '#ff3333',
            transition: 'width 0.3s'
          }} />
        </div>
      </Html>

      {/* Label */}
      <Text
        position={[agent.position.x, agent.position.y + 2.8, agent.position.z]}
        fontSize={0.3}
        color="#ffffff"
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.02}
        outlineColor="#000000"
      >
        {agent.name}
      </Text>
    </group>
  );
}

/* ─── Block Entity 3D ──────────────────────────── */
function BlockMesh({ block, onAnimateComplete }: { block: BlockEntry; onAnimateComplete: (id: string) => void }) {
  const meshRef = useRef<THREE.Mesh>(null!);
  const startRef = useRef(Date.now());
  const duration = 3000;
  const targetY = -6;

  useFrame(() => {
    if (!meshRef.current) return;
    const elapsed = Date.now() - startRef.current;
    const t = Math.min(elapsed / duration, 1);

    // Fall from top of arena to bottom
    meshRef.current.position.y = 8 - t * (8 - targetY);
    // Fade out near the end
    const opacity = t > 0.7 ? 1 - (t - 0.7) / 0.3 : 1;
    (meshRef.current.material as THREE.MeshBasicMaterial).opacity = opacity;

    if (t >= 1) {
      onAnimateComplete(block.id);
    }
  });

  return (
    <mesh ref={meshRef} position={[block.position.x, 8, block.position.z]}>
      <boxGeometry args={[0.6, 0.6, 0.6]} />
      <meshBasicMaterial
        color={block.status === 'confirmed' ? '#00cc66' : block.status === 'pending' ? '#ffaa00' : '#ff3333'}
        transparent
        opacity={1}
      />
    </mesh>
  );
}

/* ─── Transaction Beam ─────────────────────────── */
function TxBeam({ from, to, onComplete }: { from: THREE.Vector3; to: THREE.Vector3; onComplete: () => void }) {
  const points = React.useMemo(() => {
    const mid = new THREE.Vector3().addVectors(from, to).multiplyScalar(0.5);
    mid.y += 3;
    return new THREE.CatmullRomCurve3([from, mid, to]).getPoints(20);
  }, [from, to]);

  return (
    <Line
      points={points}
      color="#00ddff"
      lineWidth={1}
      transparent
      opacity={0.6}
    />
  );
}

/* ─── Main Scene ───────────────────────────────── */
function ArenaScene({ selectedAgentId, onAgentSelect }: {
  selectedAgentId: string | null;
  onAgentSelect: (id: string) => void;
}) {
  const agents = useAgentStore((s) => s.agents);
  const blocks = useBlockStore((s) => s.recentBlocks);
  const removeBlock = useBlockStore((s) => s.removeBlock);
  const { camera, gl } = useThree();

  // Camera controls
  useEffect(() => {
    camera.position.set(0, 12, 18);
    camera.lookAt(0, 0, 0);
  }, [camera]);

  const handleBlockComplete = useCallback((id: string) => {
    removeBlock(id);
  }, [removeBlock]);

  const handleClickEmpty = useCallback(() => {
    // Deselect on background click
    onAgentSelect('');
  }, [onAgentSelect]);

  return (
    <>
      {/* Lights */}
      <ambientLight intensity={0.3} />
      <directionalLight position={[10, 20, 10]} intensity={1} castShadow />
      <directionalLight position={[-10, 10, -10]} intensity={0.4} color="#4488ff" />
      <pointLight position={[0, 10, 0]} intensity={0.3} color="#00aaff" />

      {/* Arena */}
      <ArenaFloor />
      <ArenaBoundary />

      {/* Agents */}
      {agents.map((agent) => (
        <AgentMesh
          key={agent.id}
          agent={agent}
          isSelected={selectedAgentId === agent.id}
          onClick={onAgentSelect}
        />
      ))}

      {/* Blocks falling */}
      {blocks.map((block) => (
        <BlockMesh
          key={block.id}
          block={block}
          onAnimateComplete={handleBlockComplete}
        />
      ))}

      {/* Particle system for activity */}
      <ParticleFlows count={200} color="#4488ff" />

      {/* Camera Orbit Controls */}
      <OrbitControls
        enablePan={true}
        enableZoom={true}
        enableRotate={true}
        minDistance={5}
        maxDistance={40}
        maxPolarAngle={Math.PI / 2.1}
      />
    </>
  );
}

/* ─── Scene Manager Component ──────────────────── */
export function SceneManager({ selectedAgentId, onAgentSelect, onAgentDeselect }: SceneManagerProps) {
  const handleSelect = useCallback((id: string) => {
    if (id === '') {
      onAgentDeselect();
    } else {
      onAgentSelect(id);
    }
  }, [onAgentSelect, onAgentDeselect]);

  return (
    <Canvas
      shadows
      camera={{ position: [0, 12, 18], fov: 60 }}
      gl={{
        antialias: true,
        toneMapping: THREE.ACESFilmicToneMapping,
        toneMappingExposure: 1.2,
      }}
      style={{ width: '100%', height: '100%' }}
    >
      <color attach="background" args={['#050510']} />
      <fog attach="fog" args={['#050510', 25, 50]} />
      <ArenaScene
        selectedAgentId={selectedAgentId}
        onAgentSelect={handleSelect}
      />
      <CameraController
        selectedAgentId={selectedAgentId}
        agents={useAgentStore.getState().agents}
      />
    </Canvas>
  );
}