'use client';

import React, { useRef, useMemo, Suspense } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { Points, PointMaterial, OrbitControls } from '@react-three/drei';
import * as THREE from 'three';

// Quantum Particle Field
function QuantumParticles({ count = 5000 }: { count?: number }) {
  const ref = useRef<THREE.Points>(null);
  
  const positions = useMemo(() => {
    const positions = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const radius = 5 + Math.random() * 10;
      
      positions[i * 3] = radius * Math.sin(phi) * Math.cos(theta);
      positions[i * 3 + 1] = radius * Math.sin(phi) * Math.sin(theta);
      positions[i * 3 + 2] = radius * Math.cos(phi);
    }
    return positions;
  }, [count]);
  
  const colors = useMemo(() => {
    const colors = new Float32Array(count * 3);
    const color1 = new THREE.Color('#00f3ff');
    const color2 = new THREE.Color('#a855f7');
    const color3 = new THREE.Color('#ec4899');
    
    for (let i = 0; i < count; i++) {
      const t = Math.random();
      const color = t < 0.33 ? color1 : t < 0.66 ? color2 : color3;
      colors[i * 3] = color.r;
      colors[i * 3 + 1] = color.g;
      colors[i * 3 + 2] = color.b;
    }
    return colors;
  }, [count]);
  
  useFrame((state) => {
    if (ref.current) {
      ref.current.rotation.x = state.clock.elapsedTime * 0.02;
      ref.current.rotation.y = state.clock.elapsedTime * 0.03;
      
      // Pulsating effect
      const scale = 1 + Math.sin(state.clock.elapsedTime * 0.5) * 0.05;
      ref.current.scale.setScalar(scale);
    }
  });
  
  return (
    <Points ref={ref} positions={positions} colors={colors} stride={3}>
      <PointMaterial
        transparent
        vertexColors
        size={0.05}
        sizeAttenuation={true}
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </Points>
  );
}

// Quantum Ring
function QuantumRing({ radius = 4, color = '#00f3ff', rotationSpeed = 0.5 }: {
  radius?: number;
  color?: string;
  rotationSpeed?: number;
}) {
  const ref = useRef<THREE.Mesh>(null);
  
  useFrame((state) => {
    if (ref.current) {
      ref.current.rotation.z = state.clock.elapsedTime * rotationSpeed;
    }
  });
  
  return (
    <mesh ref={ref}>
      <torusGeometry args={[radius, 0.02, 16, 100]} />
      <meshBasicMaterial color={color} transparent opacity={0.6} />
    </mesh>
  );
}

// Quantum Core
function QuantumCore() {
  const ref = useRef<THREE.Mesh>(null);
  
  useFrame((state) => {
    if (ref.current) {
      ref.current.rotation.x = state.clock.elapsedTime * 0.5;
      ref.current.rotation.y = state.clock.elapsedTime * 0.3;
      
      // Pulsating scale
      const scale = 1 + Math.sin(state.clock.elapsedTime * 2) * 0.1;
      ref.current.scale.setScalar(scale);
    }
  });
  
  return (
    <mesh ref={ref}>
      <icosahedronGeometry args={[1, 2]} />
      <meshBasicMaterial 
        color="#00f3ff" 
        wireframe 
        transparent 
        opacity={0.8}
      />
    </mesh>
  );
}

// Data Streams
function DataStreams({ count = 50 }: { count?: number }) {
  const ref = useRef<THREE.Group>(null);
  
  const streams = useMemo(() => {
    return Array.from({ length: count }, (_, i) => ({
      position: [
        (Math.random() - 0.5) * 20,
        (Math.random() - 0.5) * 20,
        (Math.random() - 0.5) * 20,
      ] as [number, number, number],
      speed: 0.5 + Math.random() * 2,
      length: 0.5 + Math.random() * 1.5,
      color: ['#00f3ff', '#a855f7', '#00ff9d'][Math.floor(Math.random() * 3)]
    }));
  }, [count]);
  
  useFrame((state) => {
    if (ref.current) {
      ref.current.children.forEach((child, i) => {
        const stream = streams[i];
        (child as THREE.Mesh).position.z += stream.speed * 0.1;
        
        if ((child as THREE.Mesh).position.z > 10) {
          (child as THREE.Mesh).position.z = -10;
        }
      });
    }
  });
  
  return (
    <group ref={ref}>
      {streams.map((stream, i) => (
        <mesh key={i} position={stream.position}>
          <boxGeometry args={[0.02, 0.02, stream.length]} />
          <meshBasicMaterial color={stream.color} transparent opacity={0.6} />
        </mesh>
      ))}
    </group>
  );
}

// Floating Orbs
function FloatingOrbs({ count = 20 }: { count?: number }) {
  const orbs = useMemo(() => {
    return Array.from({ length: count }, () => ({
      position: [
        (Math.random() - 0.5) * 15,
        (Math.random() - 0.5) * 15,
        (Math.random() - 0.5) * 15,
      ] as [number, number, number],
      scale: 0.1 + Math.random() * 0.3,
      color: ['#00f3ff', '#a855f7', '#ec4899', '#00ff9d'][Math.floor(Math.random() * 4)],
      floatSpeed: 0.5 + Math.random() * 1.5,
      floatAmplitude: 0.5 + Math.random() * 1.5
    }));
  }, [count]);
  
  return (
    <group>
      {orbs.map((orb, i) => (
        <FloatingOrb key={i} {...orb} index={i} />
      ))}
    </group>
  );
}

function FloatingOrb({ 
  position, 
  scale, 
  color, 
  floatSpeed, 
  floatAmplitude, 
  index 
}: {
  position: [number, number, number];
  scale: number;
  color: string;
  floatSpeed: number;
  floatAmplitude: number;
  index: number;
}) {
  const ref = useRef<THREE.Mesh>(null);
  
  useFrame((state) => {
    if (ref.current) {
      ref.current.position.y = position[1] + Math.sin(state.clock.elapsedTime * floatSpeed + index) * floatAmplitude;
      ref.current.position.x = position[0] + Math.cos(state.clock.elapsedTime * floatSpeed * 0.5 + index) * floatAmplitude * 0.5;
    }
  });
  
  return (
    <mesh ref={ref} position={position} scale={scale}>
      <sphereGeometry args={[1, 16, 16]} />
      <meshBasicMaterial color={color} transparent opacity={0.4} />
    </mesh>
  );
}

// Camera Controller
function CameraController() {
  const { camera } = useThree();
  
  useFrame((state) => {
    // Subtle camera movement
    camera.position.x = Math.sin(state.clock.elapsedTime * 0.1) * 0.5;
    camera.position.y = Math.cos(state.clock.elapsedTime * 0.1) * 0.5;
    camera.lookAt(0, 0, 0);
  });
  
  return null;
}

// Main Scene Component
export default function QuantumScene() {
  return (
    <div className="absolute inset-0 w-full h-full">
      <Canvas
        camera={{ position: [0, 0, 12], fov: 60 }}
        gl={{ antialias: true, alpha: true }}
        style={{ background: 'transparent' }}
      >
        <Suspense fallback={null}>
          <ambientLight intensity={0.5} />
          <pointLight position={[10, 10, 10]} intensity={1} />
          
          {/* Quantum Core */}
          <QuantumCore />
          
          {/* Orbital Rings */}
          <QuantumRing radius={3} color="#00f3ff" rotationSpeed={0.5} />
          <group rotation={[Math.PI / 4, 0, 0]}>
            <QuantumRing radius={3.5} color="#a855f7" rotationSpeed={-0.3} />
          </group>
          <group rotation={[0, Math.PI / 4, Math.PI / 4]}>
            <QuantumRing radius={4} color="#ec4899" rotationSpeed={0.4} />
          </group>
          
          {/* Particle Field */}
          <QuantumParticles count={3000} />
          
          {/* Data Streams */}
          <DataStreams count={30} />
          
          {/* Floating Orbs */}
          <FloatingOrbs count={15} />
          
          {/* Camera Animation */}
          <CameraController />
          
          {/* Optional: Interactive controls */}
          <OrbitControls 
            enableZoom={false} 
            enablePan={false}
            autoRotate
            autoRotateSpeed={0.5}
          />
        </Suspense>
      </Canvas>
    </div>
  );
}

// Simplified version for lighter load
export function QuantumSceneLight() {
  return (
    <div className="absolute inset-0 w-full h-full">
      <Canvas
        camera={{ position: [0, 0, 8], fov: 60 }}
        gl={{ antialias: true, alpha: true }}
        style={{ background: 'transparent' }}
      >
        <Suspense fallback={null}>
          <ambientLight intensity={0.3} />
          
          {/* Just particles for lighter performance */}
          <QuantumParticles count={1500} />
          
          {/* Single ring */}
          <QuantumRing radius={3} color="#00f3ff" rotationSpeed={0.3} />
          
          <OrbitControls 
            enableZoom={false} 
            enablePan={false}
            autoRotate
            autoRotateSpeed={0.3}
          />
        </Suspense>
      </Canvas>
    </div>
  );
}
