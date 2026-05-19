'use client';

import React, { useState, useRef, useEffect } from 'react';
import { motion, useMotionValue, useTransform, useSpring, AnimatePresence } from 'framer-motion';

interface RotatingCube3DProps {
  size?: number;
  autoRotate?: boolean;
  rotateSpeed?: number;
  faces?: React.ReactNode[];
  className?: string;
  faceColors?: string[];
  glowColor?: string;
}

/**
 * RotatingCube3D - Interactive 3D rotating cube with customizable faces
 */
export const RotatingCube3D: React.FC<RotatingCube3DProps> = ({
  size = 200,
  autoRotate = true,
  rotateSpeed = 20,
  faces,
  className = '',
  faceColors = [
    'linear-gradient(135deg, #00ffea33, #00ffea11)',
    'linear-gradient(135deg, #ff00ff33, #ff00ff11)',
    'linear-gradient(135deg, #ffd93d33, #ffd93d11)',
    'linear-gradient(135deg, #00ff8833, #00ff8811)',
    'linear-gradient(135deg, #ff6b6b33, #ff6b6b11)',
    'linear-gradient(135deg, #a855f733, #a855f711)',
  ],
  glowColor = '#00ffea',
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  
  const rotateX = useMotionValue(0);
  const rotateY = useMotionValue(0);
  const autoRotateY = useMotionValue(0);
  
  const springX = useSpring(rotateX, { stiffness: 100, damping: 30 });
  const springY = useSpring(rotateY, { stiffness: 100, damping: 30 });

  // Auto rotation
  useEffect(() => {
    if (!autoRotate || isDragging) return;
    
    let animationId: number;
    let lastTime = 0;
    
    const animate = (time: number) => {
      if (lastTime) {
        const delta = (time - lastTime) / 1000;
        autoRotateY.set(autoRotateY.get() + delta * rotateSpeed);
      }
      lastTime = time;
      animationId = requestAnimationFrame(animate);
    };
    
    animationId = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animationId);
  }, [autoRotate, isDragging, rotateSpeed, autoRotateY]);

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width - 0.5) * 360;
    const y = ((e.clientY - rect.top) / rect.height - 0.5) * -360;
    rotateX.set(y);
    rotateY.set(x);
  };

  const halfSize = size / 2;
  const defaultFaces = [
    <div key="front" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#00ffea', fontSize: '24px', fontWeight: 'bold' }}>FRONT</div>,
    <div key="back" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#ff00ff', fontSize: '24px', fontWeight: 'bold' }}>BACK</div>,
    <div key="right" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#ffd93d', fontSize: '24px', fontWeight: 'bold' }}>RIGHT</div>,
    <div key="left" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#00ff88', fontSize: '24px', fontWeight: 'bold' }}>LEFT</div>,
    <div key="top" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#ff6b6b', fontSize: '24px', fontWeight: 'bold' }}>TOP</div>,
    <div key="bottom" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#a855f7', fontSize: '24px', fontWeight: 'bold' }}>BOTTOM</div>,
  ];

  const cubeFaces = faces || defaultFaces;
  
  const faceTransforms = [
    `rotateY(0deg) translateZ(${halfSize}px)`, // front
    `rotateY(180deg) translateZ(${halfSize}px)`, // back
    `rotateY(90deg) translateZ(${halfSize}px)`, // right
    `rotateY(-90deg) translateZ(${halfSize}px)`, // left
    `rotateX(90deg) translateZ(${halfSize}px)`, // top
    `rotateX(-90deg) translateZ(${halfSize}px)`, // bottom
  ];

  return (
    <div
      ref={containerRef}
      className={`rotating-cube-3d ${className}`}
      onMouseDown={() => setIsDragging(true)}
      onMouseUp={() => setIsDragging(false)}
      onMouseLeave={() => setIsDragging(false)}
      onMouseMove={handleMouseMove}
      style={{
        width: size,
        height: size,
        perspective: '1000px',
        cursor: isDragging ? 'grabbing' : 'grab',
      }}
    >
      <motion.div
        className="cube"
        style={{
          width: '100%',
          height: '100%',
          position: 'relative',
          transformStyle: 'preserve-3d',
          rotateX: isDragging ? springX : 0,
          rotateY: isDragging ? springY : autoRotateY,
        }}
      >
        {cubeFaces.map((face, i) => (
          <div
            key={i}
            style={{
              position: 'absolute',
              width: size,
              height: size,
              background: faceColors[i],
              border: `1px solid ${glowColor}44`,
              backdropFilter: 'blur(10px)',
              transform: faceTransforms[i],
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: `0 0 20px ${glowColor}22 inset`,
            }}
          >
            {face}
          </div>
        ))}
      </motion.div>
    </div>
  );
};

/**
 * MorphingShape - Shape that morphs between different forms
 */
export const MorphingShape: React.FC<{
  size?: number;
  colors?: string[];
  morphSpeed?: number;
  className?: string;
}> = ({
  size = 200,
  colors = ['#00ffea', '#ff00ff', '#ffd93d', '#00ff88'],
  morphSpeed = 3,
  className = '',
}) => {
  const [shapeIndex, setShapeIndex] = useState(0);

  const shapes = [
    'polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%)', // diamond
    'polygon(50% 0%, 100% 38%, 82% 100%, 18% 100%, 0% 38%)', // pentagon
    'polygon(25% 0%, 75% 0%, 100% 50%, 75% 100%, 25% 100%, 0% 50%)', // hexagon
    'polygon(50% 0%, 90% 20%, 100% 60%, 75% 100%, 25% 100%, 0% 60%, 10% 20%)', // heptagon
    'polygon(30% 0%, 70% 0%, 100% 30%, 100% 70%, 70% 100%, 30% 100%, 0% 70%, 0% 30%)', // octagon
    'circle(50%)', // circle
    'polygon(50% 0%, 61% 35%, 98% 35%, 68% 57%, 79% 91%, 50% 70%, 21% 91%, 32% 57%, 2% 35%, 39% 35%)', // star
  ];

  useEffect(() => {
    const interval = setInterval(() => {
      setShapeIndex((prev) => (prev + 1) % shapes.length);
    }, morphSpeed * 1000);
    return () => clearInterval(interval);
  }, [morphSpeed, shapes.length]);

  return (
    <motion.div
      className={`morphing-shape ${className}`}
      animate={{
        clipPath: shapes[shapeIndex],
        background: `linear-gradient(135deg, ${colors[shapeIndex % colors.length]}, ${colors[(shapeIndex + 1) % colors.length]})`,
      }}
      transition={{
        duration: morphSpeed * 0.8,
        ease: 'easeInOut',
      }}
      style={{
        width: size,
        height: size,
        boxShadow: `0 0 40px ${colors[shapeIndex % colors.length]}66`,
      }}
    />
  );
};

/**
 * PulsatingOrb - Glowing orb with pulsating effect
 */
export const PulsatingOrb: React.FC<{
  size?: number;
  color?: string;
  pulseSpeed?: number;
  className?: string;
}> = ({
  size = 100,
  color = '#00ffea',
  pulseSpeed = 2,
  className = '',
}) => {
  return (
    <motion.div
      className={`pulsating-orb ${className}`}
      animate={{
        scale: [1, 1.1, 1],
        boxShadow: [
          `0 0 ${size * 0.2}px ${color}, 0 0 ${size * 0.4}px ${color}66, 0 0 ${size * 0.6}px ${color}33, inset 0 0 ${size * 0.2}px ${color}`,
          `0 0 ${size * 0.4}px ${color}, 0 0 ${size * 0.6}px ${color}66, 0 0 ${size * 0.8}px ${color}33, inset 0 0 ${size * 0.3}px ${color}`,
          `0 0 ${size * 0.2}px ${color}, 0 0 ${size * 0.4}px ${color}66, 0 0 ${size * 0.6}px ${color}33, inset 0 0 ${size * 0.2}px ${color}`,
        ],
      }}
      transition={{
        duration: pulseSpeed,
        repeat: Infinity,
        ease: 'easeInOut',
      }}
      style={{
        width: size,
        height: size,
        borderRadius: '50%',
        background: `radial-gradient(circle at 30% 30%, ${color}, ${color}88 50%, ${color}44 100%)`,
      }}
    />
  );
};

/**
 * Cube3DShowcase - Display 3D shape effects
 */
export const Cube3DShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`cube-3d-showcase ${className}`} style={{
      background: 'linear-gradient(180deg, #0a0a1a 0%, #1a0a2e 100%)',
      padding: '4rem 2rem',
      borderRadius: '1rem',
    }}>
      <div className="text-center mb-8">
        <h2 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-purple-500">
          3D QUANTUM OBJECTS
        </h2>
        <p className="text-gray-400 mt-2">Interactive 3D elements - drag to rotate</p>
      </div>

      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        flexWrap: 'wrap',
        gap: '4rem',
      }}>
        {/* 3D Cube */}
        <div style={{ textAlign: 'center' }}>
          <RotatingCube3D
            size={200}
            faces={[
              <div key="1" style={{ color: '#00ffea', fontSize: '48px' }}>⚡</div>,
              <div key="2" style={{ color: '#ff00ff', fontSize: '48px' }}>🔮</div>,
              <div key="3" style={{ color: '#ffd93d', fontSize: '48px' }}>💎</div>,
              <div key="4" style={{ color: '#00ff88', fontSize: '48px' }}>🚀</div>,
              <div key="5" style={{ color: '#ff6b6b', fontSize: '48px' }}>🤖</div>,
              <div key="6" style={{ color: '#a855f7', fontSize: '48px' }}>🌐</div>,
            ]}
          />
          <p style={{ color: '#00ffea', marginTop: '1rem', fontFamily: 'monospace' }}>QUANTUM CUBE</p>
        </div>

        {/* Morphing Shape */}
        <div style={{ textAlign: 'center' }}>
          <MorphingShape size={200} />
          <p style={{ color: '#ff00ff', marginTop: '1rem', fontFamily: 'monospace' }}>MORPHING ENTITY</p>
        </div>

        {/* Pulsating Orbs */}
        <div style={{ textAlign: 'center' }}>
          <div style={{ display: 'flex', gap: '1rem', marginBottom: '1rem' }}>
            <PulsatingOrb size={60} color="#00ffea" pulseSpeed={2} />
            <PulsatingOrb size={80} color="#ff00ff" pulseSpeed={2.5} />
            <PulsatingOrb size={60} color="#ffd93d" pulseSpeed={3} />
          </div>
          <p style={{ color: '#ffd93d', fontFamily: 'monospace' }}>QUANTUM ORBS</p>
        </div>
      </div>
    </div>
  );
};

export default RotatingCube3D;
