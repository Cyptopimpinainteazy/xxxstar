'use client';

import React, { useRef, useEffect, useState } from 'react';
import { motion, useMotionValue, useTransform, useSpring } from 'framer-motion';

interface CyberGridProps {
  lineColor?: string;
  glowColor?: string;
  speed?: number;
  className?: string;
  children?: React.ReactNode;
  perspective?: 'low' | 'medium' | 'high';
}

/**
 * CyberGrid - Retro 80s synthwave perspective grid
 */
export const CyberGrid: React.FC<CyberGridProps> = ({
  lineColor = '#ff00ff',
  glowColor = '#ff00ff',
  speed = 2,
  className = '',
  children,
  perspective = 'medium',
}) => {
  const gridRef = useRef<HTMLDivElement>(null);
  const [offset, setOffset] = useState(0);

  useEffect(() => {
    let animationId: number;
    let lastTime = 0;
    
    const animate = (time: number) => {
      if (lastTime) {
        const delta = (time - lastTime) / 1000;
        setOffset(prev => (prev + delta * speed * 50) % 100);
      }
      lastTime = time;
      animationId = requestAnimationFrame(animate);
    };
    
    animationId = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animationId);
  }, [speed]);

  const perspectiveValues = {
    low: { transform: 'rotateX(60deg)', top: '70%' },
    medium: { transform: 'rotateX(70deg)', top: '60%' },
    high: { transform: 'rotateX(80deg)', top: '50%' },
  };

  const pv = perspectiveValues[perspective];

  return (
    <div
      ref={gridRef}
      className={`cyber-grid ${className}`}
      style={{
        position: 'relative',
        width: '100%',
        minHeight: '500px',
        overflow: 'hidden',
        background: 'linear-gradient(180deg, #000000 0%, #0a0015 50%, #150025 100%)',
      }}
    >
      {/* Sun */}
      <div
        style={{
          position: 'absolute',
          width: '300px',
          height: '300px',
          left: '50%',
          top: '30%',
          transform: 'translate(-50%, -50%)',
          borderRadius: '50%',
          background: `
            linear-gradient(180deg, 
              #ff6b35 0%, 
              #ff0066 30%, 
              #9900ff 60%, 
              transparent 100%
            )
          `,
          boxShadow: `
            0 0 80px #ff0066,
            0 0 120px #ff006666,
            0 0 200px #ff006633
          `,
          clipPath: 'polygon(0 0, 100% 0, 100% 50%, 95% 52%, 90% 50%, 85% 52%, 80% 50%, 75% 52%, 70% 50%, 65% 52%, 60% 50%, 55% 52%, 50% 50%, 45% 52%, 40% 50%, 35% 52%, 30% 50%, 25% 52%, 20% 50%, 15% 52%, 10% 50%, 5% 52%, 0% 50%)',
        }}
      />

      {/* Grid container */}
      <div
        style={{
          position: 'absolute',
          width: '200%',
          height: '100%',
          left: '-50%',
          top: pv.top,
          transform: pv.transform,
          transformOrigin: 'center top',
          perspective: '1000px',
        }}
      >
        {/* Horizontal lines */}
        <svg
          style={{
            position: 'absolute',
            width: '100%',
            height: '100%',
          }}
        >
          <defs>
            <linearGradient id="gridLineGradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor={lineColor} stopOpacity="1" />
              <stop offset="50%" stopColor={lineColor} stopOpacity="0.5" />
              <stop offset="100%" stopColor={lineColor} stopOpacity="0" />
            </linearGradient>
            <filter id="gridGlow">
              <feGaussianBlur stdDeviation="2" result="coloredBlur" />
              <feMerge>
                <feMergeNode in="coloredBlur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Horizontal lines moving towards viewer */}
          {[...Array(20)].map((_, i) => {
            const y = ((i * 5 + offset) % 100);
            return (
              <line
                key={`h-${i}`}
                x1="0%"
                y1={`${y}%`}
                x2="100%"
                y2={`${y}%`}
                stroke="url(#gridLineGradient)"
                strokeWidth={1 + (y / 100) * 2}
                filter="url(#gridGlow)"
                opacity={0.3 + (y / 100) * 0.7}
              />
            );
          })}

          {/* Vertical lines */}
          {[...Array(21)].map((_, i) => (
            <line
              key={`v-${i}`}
              x1={`${i * 5}%`}
              y1="0%"
              x2={`${25 + (i - 10) * 2.5}%`}
              y2="100%"
              stroke={lineColor}
              strokeWidth="1"
              filter="url(#gridGlow)"
              opacity="0.4"
            />
          ))}
        </svg>
      </div>

      {/* Gradient overlay */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          background: 'linear-gradient(180deg, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 40%, rgba(0,0,20,0.5) 100%)',
          pointerEvents: 'none',
        }}
      />

      {/* Side glow */}
      <div
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          bottom: 0,
          width: '100px',
          background: `linear-gradient(90deg, ${glowColor}22, transparent)`,
          pointerEvents: 'none',
        }}
      />
      <div
        style={{
          position: 'absolute',
          right: 0,
          top: 0,
          bottom: 0,
          width: '100px',
          background: `linear-gradient(-90deg, ${glowColor}22, transparent)`,
          pointerEvents: 'none',
        }}
      />

      {/* Content */}
      <div style={{ position: 'relative', zIndex: 10 }}>
        {children}
      </div>
    </div>
  );
};

/**
 * SynthwaveText - 80s synthwave styled text
 */
export const SynthwaveText: React.FC<{
  text: string;
  fontSize?: string;
  className?: string;
}> = ({ text, fontSize = 'clamp(3rem, 10vw, 8rem)', className = '' }) => {
  return (
    <motion.h1
      className={`synthwave-text ${className}`}
      style={{
        fontSize,
        fontWeight: 900,
        fontFamily: "'Orbitron', sans-serif",
        textTransform: 'uppercase',
        letterSpacing: '0.1em',
        background: 'linear-gradient(180deg, #fff 0%, #ff00ff 50%, #0ff 100%)',
        WebkitBackgroundClip: 'text',
        backgroundClip: 'text',
        color: 'transparent',
        textShadow: `
          0 0 20px #ff00ff,
          0 0 40px #ff00ff88,
          0 0 80px #ff00ff44,
          0 5px 0 #0ff,
          0 10px 0 #ff0066
        `,
        position: 'relative',
      }}
      animate={{
        textShadow: [
          '0 0 20px #ff00ff, 0 0 40px #ff00ff88, 0 0 80px #ff00ff44',
          '0 0 30px #0ff, 0 0 60px #0ff88, 0 0 100px #0ff44',
          '0 0 20px #ff00ff, 0 0 40px #ff00ff88, 0 0 80px #ff00ff44',
        ],
      }}
      transition={{ duration: 3, repeat: Infinity }}
    >
      {text}
    </motion.h1>
  );
};

/**
 * NeonSign - Flickering neon sign effect
 */
export const NeonSign: React.FC<{
  text: string;
  color?: string;
  className?: string;
}> = ({ text, color = '#ff00ff', className = '' }) => {
  return (
    <motion.div
      className={`neon-sign ${className}`}
      animate={{
        opacity: [1, 0.8, 1, 0.9, 1, 0.7, 1],
        filter: [
          `drop-shadow(0 0 5px ${color}) drop-shadow(0 0 20px ${color})`,
          `drop-shadow(0 0 2px ${color}) drop-shadow(0 0 10px ${color})`,
          `drop-shadow(0 0 5px ${color}) drop-shadow(0 0 20px ${color})`,
          `drop-shadow(0 0 3px ${color}) drop-shadow(0 0 15px ${color})`,
          `drop-shadow(0 0 5px ${color}) drop-shadow(0 0 20px ${color})`,
        ],
      }}
      transition={{
        duration: 0.5,
        repeat: Infinity,
        repeatDelay: 3,
      }}
      style={{
        fontSize: '3rem',
        fontFamily: "'Orbitron', monospace",
        fontWeight: 'bold',
        color,
        textShadow: `0 0 10px ${color}, 0 0 20px ${color}, 0 0 40px ${color}`,
      }}
    >
      {text}
    </motion.div>
  );
};

/**
 * CyberGridShowcase - Display synthwave grid effects
 */
export const CyberGridShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`cyber-grid-showcase ${className}`}>
      <CyberGrid perspective="medium" lineColor="#ff00ff" glowColor="#ff00ff">
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '500px',
          padding: '2rem',
          textAlign: 'center',
        }}>
          <SynthwaveText text="X3" />
          
          <motion.p
            style={{
              fontSize: '1.5rem',
              color: '#0ff',
              fontFamily: "'Orbitron', monospace",
              textShadow: '0 0 20px #0ff',
              marginTop: '1rem',
            }}
          >
            YEAR 2060 • QUANTUM BLOCKCHAIN
          </motion.p>
          
          <div style={{ marginTop: '2rem', display: 'flex', gap: '2rem', flexWrap: 'wrap', justifyContent: 'center' }}>
            <NeonSign text="EVM" color="#ff00ff" />
            <NeonSign text="+" color="#ffffff" />
            <NeonSign text="SVM" color="#00ffff" />
          </div>
          
          <motion.button
            whileHover={{ 
              scale: 1.1, 
              boxShadow: '0 0 40px #ff00ff, 0 0 80px #ff00ff88',
            }}
            whileTap={{ scale: 0.95 }}
            style={{
              marginTop: '3rem',
              padding: '16px 48px',
              background: 'transparent',
              border: '3px solid #ff00ff',
              borderRadius: '0',
              color: '#ff00ff',
              fontSize: '1.2rem',
              fontFamily: "'Orbitron', monospace",
              fontWeight: 'bold',
              cursor: 'pointer',
              boxShadow: '0 0 20px #ff00ff88, inset 0 0 20px #ff00ff22',
              textShadow: '0 0 10px #ff00ff',
            }}
          >
            ENTER THE GRID
          </motion.button>
        </div>
      </CyberGrid>
    </div>
  );
};

export default CyberGrid;
