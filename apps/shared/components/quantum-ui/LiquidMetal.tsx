'use client';

import React, { useRef, useEffect, useState } from 'react';
import { motion, useMotionValue, useTransform, useSpring, AnimatePresence } from 'framer-motion';

interface LiquidMetalBlobProps {
  size?: number;
  color?: string;
  colorSecondary?: string;
  className?: string;
  morphSpeed?: number;
}

/**
 * LiquidMetalBlob - Morphing liquid metal blob with reflection
 */
export const LiquidMetalBlob: React.FC<LiquidMetalBlobProps> = ({
  size = 200,
  color = '#c0c0c0',
  colorSecondary = '#404040',
  className = '',
  morphSpeed = 4,
}) => {
  const [morphIndex, setMorphIndex] = useState(0);
  
  const morphPaths = [
    'M45.3,-76.7C58.7,-69.4,69.8,-57,77.4,-42.7C85,-28.4,89,-12.2,88.1,3.7C87.2,19.6,81.4,35.2,71.5,47.5C61.6,59.8,47.6,68.8,32.5,75.3C17.4,81.8,1.2,85.7,-15.3,84.2C-31.8,82.7,-48.6,75.8,-62.7,64.6C-76.8,53.4,-88.1,38,-91.7,20.8C-95.3,3.7,-91.2,-15.2,-83,-31.8C-74.8,-48.4,-62.6,-62.8,-47.8,-69.3C-33,-75.8,-16.5,-74.4,0.3,-74.9C17.1,-75.4,34.2,-77.7,45.3,-76.7Z',
    'M39.5,-67.1C51.3,-59.6,61.2,-49.2,68.5,-36.9C75.8,-24.6,80.5,-10.3,80.3,4.1C80.1,18.5,75,33,66.6,45.3C58.2,57.6,46.5,67.7,33,74C19.5,80.3,4.2,82.8,-11.4,81.2C-27,79.6,-43,73.9,-55.8,64C-68.6,54.1,-78.2,40,-83,24.2C-87.8,8.4,-87.8,-9.1,-82.4,-24.1C-77,-39.1,-66.2,-51.6,-53,-59.8C-39.8,-68,-24.2,-71.9,-9,-69.9C6.2,-67.9,27.7,-74.6,39.5,-67.1Z',
    'M44.9,-76.2C57.4,-68.2,66.2,-54.1,73,-39.1C79.8,-24.1,84.6,-8.2,83.4,7.1C82.2,22.4,75,37.1,64.6,48.3C54.2,59.5,40.6,67.2,26,72.7C11.4,78.2,-4.2,81.5,-19.6,79.5C-35,77.5,-50.2,70.2,-62.3,59C-74.4,47.8,-83.4,32.7,-86.1,16.4C-88.8,0.1,-85.2,-17.4,-77.6,-32.3C-70,-47.2,-58.4,-59.5,-44.7,-67C-31,-74.5,-15.5,-77.2,0.6,-78.2C16.7,-79.2,33.4,-78.5,44.9,-76.2Z',
    'M41.7,-71.5C54,-64.5,63.9,-53.6,71.4,-40.9C78.9,-28.2,84,-13.7,83.9,0.7C83.8,15.1,78.5,29.5,70.2,41.5C61.9,53.5,50.6,63.1,37.6,69.8C24.6,76.5,9.9,80.3,-4.5,78.8C-18.9,77.3,-33,70.5,-45.6,61.5C-58.2,52.5,-69.3,41.3,-76.1,27.5C-82.9,13.7,-85.4,-2.7,-82.3,-17.9C-79.2,-33.1,-70.5,-47.1,-58.6,-56.3C-46.7,-65.5,-31.6,-69.9,-17.3,-72.3C-3,-74.7,10.5,-75.1,23.8,-73.5C37.1,-71.9,50.2,-68.3,41.7,-71.5Z',
  ];

  useEffect(() => {
    const interval = setInterval(() => {
      setMorphIndex((prev) => (prev + 1) % morphPaths.length);
    }, morphSpeed * 1000);
    return () => clearInterval(interval);
  }, [morphSpeed, morphPaths.length]);

  return (
    <div
      className={`liquid-metal-blob ${className}`}
      style={{
        width: size,
        height: size,
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <svg
        viewBox="-100 -100 200 200"
        style={{ width: '100%', height: '100%' }}
      >
        <defs>
          {/* Chrome/metal gradient */}
          <linearGradient id="metalGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#f0f0f0" />
            <stop offset="20%" stopColor={color} />
            <stop offset="40%" stopColor="#f8f8f8" />
            <stop offset="60%" stopColor={colorSecondary} />
            <stop offset="80%" stopColor={color} />
            <stop offset="100%" stopColor="#e0e0e0" />
          </linearGradient>
          
          {/* Reflection highlight */}
          <radialGradient id="metalHighlight" cx="30%" cy="30%" r="50%">
            <stop offset="0%" stopColor="white" stopOpacity="0.8" />
            <stop offset="100%" stopColor="white" stopOpacity="0" />
          </radialGradient>
          
          {/* Drop shadow filter */}
          <filter id="metalShadow">
            <feDropShadow dx="0" dy="10" stdDeviation="10" floodOpacity="0.5" />
          </filter>
        </defs>
        
        {/* Main blob */}
        <motion.path
          d={morphPaths[morphIndex]}
          fill="url(#metalGradient)"
          filter="url(#metalShadow)"
          animate={{
            d: morphPaths[morphIndex],
          }}
          transition={{
            duration: morphSpeed,
            ease: 'easeInOut',
          }}
          style={{
            transformOrigin: 'center',
          }}
        />
        
        {/* Highlight overlay */}
        <motion.path
          d={morphPaths[morphIndex]}
          fill="url(#metalHighlight)"
          animate={{
            d: morphPaths[morphIndex],
          }}
          transition={{
            duration: morphSpeed,
            ease: 'easeInOut',
          }}
        />
      </svg>
    </div>
  );
};

/**
 * LiquidMetalText - Text with liquid metal effect
 */
export const LiquidMetalText: React.FC<{
  text: string;
  fontSize?: string;
  className?: string;
}> = ({ text, fontSize = '6rem', className = '' }) => {
  return (
    <div className={`liquid-metal-text ${className}`} style={{ position: 'relative' }}>
      <svg width="0" height="0">
        <defs>
          <linearGradient id="liquidTextGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#f0f0f0" />
            <stop offset="25%" stopColor="#c0c0c0" />
            <stop offset="50%" stopColor="#ffffff" />
            <stop offset="75%" stopColor="#808080" />
            <stop offset="100%" stopColor="#d0d0d0" />
          </linearGradient>
          <filter id="liquidTextEffect">
            <feTurbulence type="fractalNoise" baseFrequency="0.02" numOctaves="3" result="noise">
              <animate attributeName="baseFrequency" values="0.02;0.04;0.02" dur="10s" repeatCount="indefinite" />
            </feTurbulence>
            <feDisplacementMap in="SourceGraphic" in2="noise" scale="5" xChannelSelector="R" yChannelSelector="G" />
          </filter>
        </defs>
      </svg>
      
      <motion.h1
        style={{
          fontSize,
          fontWeight: 900,
          fontFamily: "'Orbitron', sans-serif",
          background: 'linear-gradient(135deg, #f0f0f0 0%, #c0c0c0 25%, #ffffff 50%, #808080 75%, #d0d0d0 100%)',
          WebkitBackgroundClip: 'text',
          backgroundClip: 'text',
          color: 'transparent',
          textShadow: '0 5px 15px rgba(0,0,0,0.3)',
          filter: 'url(#liquidTextEffect)',
        }}
        animate={{
          backgroundPosition: ['0% 0%', '100% 100%', '0% 0%'],
        }}
        transition={{
          duration: 8,
          repeat: Infinity,
          ease: 'linear',
        }}
      >
        {text}
      </motion.h1>
    </div>
  );
};

/**
 * LiquidMetalButton - Chrome/metal button with liquid effect
 */
export const LiquidMetalButton: React.FC<{
  children: React.ReactNode;
  onClick?: () => void;
  className?: string;
}> = ({ children, onClick, className = '' }) => {
  const [isPressed, setIsPressed] = useState(false);

  return (
    <motion.button
      className={`liquid-metal-button ${className}`}
      onClick={onClick}
      onMouseDown={() => setIsPressed(true)}
      onMouseUp={() => setIsPressed(false)}
      onMouseLeave={() => setIsPressed(false)}
      whileHover={{ scale: 1.05 }}
      whileTap={{ scale: 0.95 }}
      style={{
        padding: '16px 40px',
        fontSize: '18px',
        fontWeight: 'bold',
        fontFamily: "'Orbitron', monospace",
        border: 'none',
        borderRadius: '50px',
        cursor: 'pointer',
        position: 'relative',
        overflow: 'hidden',
        background: `linear-gradient(145deg, 
          #f0f0f0 0%, 
          #c0c0c0 20%, 
          #ffffff 40%, 
          #a0a0a0 60%, 
          #d0d0d0 80%, 
          #e0e0e0 100%
        )`,
        color: '#333',
        boxShadow: isPressed
          ? 'inset 0 4px 10px rgba(0,0,0,0.3), 0 2px 5px rgba(0,0,0,0.2)'
          : '0 8px 25px rgba(0,0,0,0.3), inset 0 -3px 10px rgba(0,0,0,0.1), inset 0 3px 10px rgba(255,255,255,0.5)',
        textShadow: '0 1px 2px rgba(255,255,255,0.5)',
      }}
    >
      {/* Shine animation */}
      <motion.div
        style={{
          position: 'absolute',
          top: 0,
          left: '-100%',
          width: '50%',
          height: '100%',
          background: 'linear-gradient(90deg, transparent, rgba(255,255,255,0.4), transparent)',
          transform: 'skewX(-25deg)',
        }}
        animate={{
          left: ['−100%', '200%'],
        }}
        transition={{
          duration: 2,
          repeat: Infinity,
          repeatDelay: 3,
        }}
      />
      
      <span style={{ position: 'relative', zIndex: 1 }}>{children}</span>
    </motion.button>
  );
};

/**
 * LiquidMetalShowcase - Display liquid metal effects
 */
export const LiquidMetalShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`liquid-metal-showcase ${className}`} style={{
      background: 'linear-gradient(180deg, #0a0a0a 0%, #1a1a2e 100%)',
      padding: '4rem 2rem',
      borderRadius: '1rem',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: '3rem',
    }}>
      <LiquidMetalText text="LIQUID METAL" />
      
      <div style={{ display: 'flex', gap: '2rem', flexWrap: 'wrap', justifyContent: 'center' }}>
        <LiquidMetalBlob size={150} color="#c0c0c0" colorSecondary="#606060" />
        <LiquidMetalBlob size={150} color="#ffd700" colorSecondary="#b8860b" />
        <LiquidMetalBlob size={150} color="#00ffea" colorSecondary="#008080" />
      </div>
      
      <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', justifyContent: 'center' }}>
        <LiquidMetalButton>CONNECT WALLET</LiquidMetalButton>
        <LiquidMetalButton>SWAP TOKENS</LiquidMetalButton>
      </div>
    </div>
  );
};

export default LiquidMetalBlob;
