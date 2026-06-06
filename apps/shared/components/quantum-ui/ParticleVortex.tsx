'use client';

import React, { useRef, useEffect, useMemo } from 'react';
import { motion } from 'framer-motion';

interface ParticleVortexProps {
  particleCount?: number;
  size?: number;
  className?: string;
  colorMode?: 'rainbow' | 'cyan' | 'gold' | 'matrix';
}

/**
 * ParticleVortex - 360 spinning particles with rainbow color animation
 * Based on the SCSS particle vortex effect with 3D transforms
 */
export const ParticleVortex: React.FC<ParticleVortexProps> = ({
  particleCount = 180,
  size = 5,
  className = '',
  colorMode = 'rainbow',
}) => {
  const containerRef = useRef<HTMLDivElement>(null);

  const particles = useMemo(() => {
    return Array.from({ length: particleCount }, (_, i) => {
      const hueStart = colorMode === 'rainbow' ? i * -1 : 
                       colorMode === 'cyan' ? 180 + (i % 60) : 
                       colorMode === 'gold' ? 40 + (i % 30) :
                       120 + (i % 40);
      const hueEnd = colorMode === 'rainbow' ? i : 
                     colorMode === 'cyan' ? 200 + (i % 40) : 
                     colorMode === 'gold' ? 50 + (i % 20) :
                     130 + (i % 30);
      
      return {
        id: i,
        delay: i * 0.005,
        hueStart,
        hueEnd,
        rotateZ: i * 45,
        rotateZEnd: i * 90,
        rotateXEnd: i * 1,
        translateX: i * 1,
        translateY: i * 1,
        translateZ: i * -0.075,
        translateXEnd: i * -3,
        translateYEnd: i * 2,
      };
    });
  }, [particleCount, colorMode]);

  return (
    <div 
      ref={containerRef}
      className={`particle-vortex-container ${className}`}
      style={{
        position: 'relative',
        width: '100%',
        height: '100%',
        minHeight: '400px',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        overflow: 'hidden',
        background: 'black',
        perspective: '1000px',
      }}
    >
      <div className="vortex-center" style={{ position: 'relative' }}>
        {particles.map((particle) => (
          <motion.div
            key={particle.id}
            className="vortex-particle"
            initial={{
              backgroundColor: `hsl(${particle.hueStart}, 100%, 50%)`,
              transform: `
                rotateZ(${particle.rotateZ}deg)
                perspective(${size * 8}px)
                translate3d(${particle.translateX}px, ${particle.translateY}px, ${particle.translateZ}px)
              `,
            }}
            animate={{
              backgroundColor: [
                `hsl(${particle.hueStart}, 100%, 50%)`,
                `hsl(${particle.hueEnd}, 100%, 70%)`,
                `hsl(${particle.hueStart}, 100%, 50%)`,
              ],
              transform: [
                `rotateZ(${particle.rotateZ}deg) perspective(${size * 8}px) translate3d(${particle.translateX}px, ${particle.translateY}px, ${particle.translateZ}px)`,
                `rotateZ(${particle.rotateZEnd}deg) rotateX(${particle.rotateXEnd}deg) perspective(${size * 3}px) translate3d(${particle.translateXEnd}px, ${particle.translateYEnd}px, ${particle.translateZ}px)`,
                `rotateZ(${particle.rotateZ}deg) perspective(${size * 8}px) translate3d(${particle.translateX}px, ${particle.translateY}px, ${particle.translateZ}px)`,
              ],
            }}
            transition={{
              duration: 3,
              ease: 'easeInOut',
              repeat: Infinity,
              repeatType: 'reverse',
              delay: particle.delay,
            }}
            style={{
              position: 'absolute',
              width: `${size}px`,
              height: `${size}px`,
              borderRadius: '50%',
              transformStyle: 'preserve-3d',
            }}
          />
        ))}
      </div>
      
      {/* Glow overlay */}
      <div 
        style={{
          position: 'absolute',
          inset: 0,
          background: 'radial-gradient(circle at center, transparent 30%, rgba(0,0,0,0.8) 70%)',
          pointerEvents: 'none',
        }}
      />
    </div>
  );
};

/**
 * ParticleVortexShowcase - Displays multiple vortex variations
 */
export const ParticleVortexShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`particle-vortex-showcase ${className}`}>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-8 p-8">
        {/* Rainbow Vortex */}
        <div className="vortex-card rounded-2xl overflow-hidden border border-white/10">
          <ParticleVortex colorMode="rainbow" particleCount={180} />
          <div className="p-4 bg-black/80 backdrop-blur-sm">
            <h3 className="text-xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-red-500 via-yellow-500 to-cyan-500">
              Rainbow Vortex
            </h3>
            <p className="text-gray-400 text-sm">360° particle spectrum animation</p>
          </div>
        </div>

        {/* Cyan Vortex */}
        <div className="vortex-card rounded-2xl overflow-hidden border border-cyan-500/30">
          <ParticleVortex colorMode="cyan" particleCount={150} />
          <div className="p-4 bg-black/80 backdrop-blur-sm">
            <h3 className="text-xl font-bold text-cyan-400">Quantum Vortex</h3>
            <p className="text-gray-400 text-sm">Entangled particle visualization</p>
          </div>
        </div>

        {/* Gold Vortex */}
        <div className="vortex-card rounded-2xl overflow-hidden border border-yellow-500/30">
          <ParticleVortex colorMode="gold" particleCount={150} />
          <div className="p-4 bg-black/80 backdrop-blur-sm">
            <h3 className="text-xl font-bold text-yellow-400">Treasury Vortex</h3>
            <p className="text-gray-400 text-sm">DeFi yield visualization</p>
          </div>
        </div>

        {/* Matrix Vortex */}
        <div className="vortex-card rounded-2xl overflow-hidden border border-green-500/30">
          <ParticleVortex colorMode="matrix" particleCount={200} />
          <div className="p-4 bg-black/80 backdrop-blur-sm">
            <h3 className="text-xl font-bold text-green-400">Matrix Vortex</h3>
            <p className="text-gray-400 text-sm">Neural network processing</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ParticleVortex;
