'use client';

import React, { useRef, useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface AuroraBorealisProps {
  intensity?: 'low' | 'medium' | 'high';
  colors?: string[];
  className?: string;
  children?: React.ReactNode;
}

/**
 * AuroraBorealis - Northern lights effect background
 */
export const AuroraBorealis: React.FC<AuroraBorealisProps> = ({
  intensity = 'medium',
  colors = ['#00ff88', '#00ffea', '#0088ff', '#8800ff', '#ff00ff'],
  className = '',
  children,
}) => {
  const waveCount = intensity === 'low' ? 3 : intensity === 'medium' ? 5 : 8;
  
  return (
    <div
      className={`aurora-borealis ${className}`}
      style={{
        position: 'relative',
        width: '100%',
        minHeight: '500px',
        overflow: 'hidden',
        background: 'linear-gradient(180deg, #000510 0%, #001020 50%, #000818 100%)',
      }}
    >
      {/* Aurora waves */}
      {[...Array(waveCount)].map((_, i) => (
        <motion.div
          key={i}
          className="aurora-wave"
          style={{
            position: 'absolute',
            width: '200%',
            height: '60%',
            left: '-50%',
            top: `${10 + i * 10}%`,
            background: `linear-gradient(90deg, 
              transparent 0%, 
              ${colors[i % colors.length]}22 20%, 
              ${colors[(i + 1) % colors.length]}44 40%, 
              ${colors[(i + 2) % colors.length]}33 60%, 
              ${colors[i % colors.length]}22 80%, 
              transparent 100%
            )`,
            filter: 'blur(40px)',
            opacity: 0.6 - i * 0.05,
            mixBlendMode: 'screen',
          }}
          animate={{
            x: ['-10%', '10%', '-10%'],
            scaleY: [1, 1.2, 0.8, 1],
            opacity: [0.4, 0.7, 0.5, 0.4],
          }}
          transition={{
            duration: 8 + i * 2,
            repeat: Infinity,
            ease: 'easeInOut',
            delay: i * 0.5,
          }}
        />
      ))}

      {/* Stars */}
      {[...Array(50)].map((_, i) => (
        <motion.div
          key={`star-${i}`}
          style={{
            position: 'absolute',
            width: `${1 + Math.random() * 2}px`,
            height: `${1 + Math.random() * 2}px`,
            borderRadius: '50%',
            background: 'white',
            left: `${Math.random() * 100}%`,
            top: `${Math.random() * 50}%`,
            boxShadow: '0 0 4px white',
          }}
          animate={{
            opacity: [0.3, 1, 0.3],
          }}
          transition={{
            duration: 2 + Math.random() * 3,
            repeat: Infinity,
            delay: Math.random() * 2,
          }}
        />
      ))}

      {/* Shooting stars */}
      <AnimatePresence>
        {[...Array(3)].map((_, i) => (
          <motion.div
            key={`shooting-${i}`}
            initial={{
              x: `${Math.random() * 50}%`,
              y: '-10%',
              opacity: 0,
            }}
            animate={{
              x: `${Math.random() * 50 + 50}%`,
              y: '60%',
              opacity: [0, 1, 0],
            }}
            transition={{
              duration: 1,
              repeat: Infinity,
              repeatDelay: 5 + Math.random() * 10,
              delay: i * 7,
            }}
            style={{
              position: 'absolute',
              width: '100px',
              height: '2px',
              background: 'linear-gradient(90deg, white, transparent)',
              transform: 'rotate(45deg)',
              borderRadius: '2px',
            }}
          />
        ))}
      </AnimatePresence>

      {/* Content */}
      <div style={{ position: 'relative', zIndex: 10 }}>
        {children}
      </div>

      {/* Ground reflection */}
      <div
        style={{
          position: 'absolute',
          bottom: 0,
          left: 0,
          right: 0,
          height: '30%',
          background: 'linear-gradient(180deg, transparent 0%, rgba(0,50,80,0.3) 100%)',
          pointerEvents: 'none',
        }}
      />
    </div>
  );
};

/**
 * CosmicDust - Floating dust particles with subtle movement
 */
export const CosmicDust: React.FC<{
  count?: number;
  color?: string;
  className?: string;
}> = ({ count = 100, color = '#ffffff', className = '' }) => {
  return (
    <div className={`cosmic-dust ${className}`} style={{ position: 'absolute', inset: 0, overflow: 'hidden', pointerEvents: 'none' }}>
      {[...Array(count)].map((_, i) => (
        <motion.div
          key={i}
          style={{
            position: 'absolute',
            width: `${1 + Math.random() * 3}px`,
            height: `${1 + Math.random() * 3}px`,
            borderRadius: '50%',
            background: color,
            left: `${Math.random() * 100}%`,
            top: `${Math.random() * 100}%`,
            opacity: 0.2 + Math.random() * 0.5,
          }}
          animate={{
            y: [0, -30, 0],
            x: [0, Math.random() * 20 - 10, 0],
            opacity: [0.2, 0.6, 0.2],
          }}
          transition={{
            duration: 10 + Math.random() * 20,
            repeat: Infinity,
            ease: 'easeInOut',
            delay: Math.random() * 10,
          }}
        />
      ))}
    </div>
  );
};

/**
 * NebulaClouds - Colorful nebula cloud formations
 */
export const NebulaClouds: React.FC<{
  className?: string;
}> = ({ className = '' }) => {
  const clouds = [
    { x: '20%', y: '30%', size: 300, color1: '#ff006620', color2: '#8800ff10' },
    { x: '70%', y: '50%', size: 400, color1: '#00ffea15', color2: '#0088ff10' },
    { x: '40%', y: '70%', size: 350, color1: '#ff880015', color2: '#ff006610' },
    { x: '80%', y: '20%', size: 250, color1: '#00ff8820', color2: '#00ffea15' },
  ];

  return (
    <div className={`nebula-clouds ${className}`} style={{ position: 'absolute', inset: 0, overflow: 'hidden', pointerEvents: 'none' }}>
      {clouds.map((cloud, i) => (
        <motion.div
          key={i}
          style={{
            position: 'absolute',
            left: cloud.x,
            top: cloud.y,
            width: cloud.size,
            height: cloud.size,
            borderRadius: '50%',
            background: `radial-gradient(ellipse at center, ${cloud.color1} 0%, ${cloud.color2} 50%, transparent 70%)`,
            filter: 'blur(60px)',
            transform: 'translate(-50%, -50%)',
          }}
          animate={{
            scale: [1, 1.2, 1],
            x: [0, 30, 0],
            y: [0, -20, 0],
          }}
          transition={{
            duration: 20 + i * 5,
            repeat: Infinity,
            ease: 'easeInOut',
          }}
        />
      ))}
    </div>
  );
};

/**
 * AuroraShowcase - Display aurora and cosmic effects
 */
export const AuroraShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`aurora-showcase ${className}`}>
      <AuroraBorealis intensity="high">
        <CosmicDust count={80} />
        <NebulaClouds />
        
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '500px',
          padding: '4rem 2rem',
          textAlign: 'center',
        }}>
          <motion.h2
            style={{
              fontSize: 'clamp(2rem, 8vw, 5rem)',
              fontWeight: 900,
              fontFamily: "'Orbitron', monospace",
              background: 'linear-gradient(135deg, #00ff88, #00ffea, #8800ff, #ff00ff)',
              WebkitBackgroundClip: 'text',
              backgroundClip: 'text',
              color: 'transparent',
              textShadow: '0 0 60px rgba(0,255,136,0.5)',
              marginBottom: '1rem',
            }}
            animate={{
              textShadow: [
                '0 0 60px rgba(0,255,136,0.5)',
                '0 0 80px rgba(0,255,234,0.6)',
                '0 0 60px rgba(136,0,255,0.5)',
                '0 0 60px rgba(0,255,136,0.5)',
              ],
            }}
            transition={{ duration: 5, repeat: Infinity }}
          >
            AURORA GENESIS
          </motion.h2>
          
          <p style={{
            fontSize: '1.2rem',
            color: '#00ffea',
            maxWidth: '600px',
            lineHeight: 1.6,
            textShadow: '0 0 20px rgba(0,255,234,0.5)',
          }}>
            Experience the cosmic beauty of X3 Chain&apos;s quantum consensus - 
            where blockchain meets the northern lights
          </p>
          
          <motion.div
            style={{
              marginTop: '2rem',
              display: 'flex',
              gap: '1rem',
              flexWrap: 'wrap',
              justifyContent: 'center',
            }}
          >
            <motion.button
              whileHover={{ scale: 1.05, boxShadow: '0 0 40px rgba(0,255,136,0.6)' }}
              whileTap={{ scale: 0.95 }}
              style={{
                padding: '16px 32px',
                background: 'linear-gradient(135deg, #00ff88, #00ffea)',
                border: 'none',
                borderRadius: '30px',
                color: '#000',
                fontWeight: 'bold',
                fontSize: '1rem',
                cursor: 'pointer',
                boxShadow: '0 0 30px rgba(0,255,136,0.4)',
              }}
            >
              ENTER THE AURORA
            </motion.button>
            
            <motion.button
              whileHover={{ scale: 1.05, borderColor: '#00ffea' }}
              whileTap={{ scale: 0.95 }}
              style={{
                padding: '16px 32px',
                background: 'transparent',
                border: '2px solid #00ff88',
                borderRadius: '30px',
                color: '#00ff88',
                fontWeight: 'bold',
                fontSize: '1rem',
                cursor: 'pointer',
              }}
            >
              LEARN MORE
            </motion.button>
          </motion.div>
        </div>
      </AuroraBorealis>
    </div>
  );
};

export default AuroraBorealis;
