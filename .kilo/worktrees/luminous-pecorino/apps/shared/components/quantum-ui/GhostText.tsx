'use client';

import React, { useEffect, useState } from 'react';
import { motion } from 'framer-motion';

interface GhostTextProps {
  text: string;
  className?: string;
  ghostSpeed?: number;
  fontSize?: string;
  textColor?: string;
  ghostColor?: string;
}

/**
 * GhostText - Text with ghost passing through using mix-blend-mode exclusion
 * Creates a spooky reveal effect as ghost moves across the text
 */
export const GhostText: React.FC<GhostTextProps> = ({
  text,
  className = '',
  ghostSpeed = 4,
  fontSize = '25vmin',
  textColor = '#000',
  ghostColor = '#e7e6e6',
}) => {
  return (
    <div
      className={`ghost-text-container ${className}`}
      style={{
        width: '100%',
        height: '100%',
        minHeight: '300px',
        display: 'grid',
        placeItems: 'center',
        overflow: 'hidden',
        background: ghostColor,
        backgroundImage: 'url("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAYAAACp8Z5+AAAAEklEQVQImWNgYGD4z0AswK4SAFXuAf8EPy+xAAAAAElFTkSuQmCC")',
        position: 'relative',
      }}
    >
      {/* Title */}
      <div
        style={{
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
        }}
      >
        <h1
          style={{
            fontSize,
            fontWeight: 900,
            fontFamily: "'Montserrat', 'Orbitron', sans-serif",
            color: textColor,
            margin: 0,
            userSelect: 'none',
          }}
        >
          {text}
        </h1>
      </div>

      {/* Ghost */}
      <motion.div
        className="ghost"
        style={{
          width: '8vmin',
          height: '12vmin',
          backgroundColor: ghostColor,
          backgroundImage: `
            radial-gradient(ellipse at 35% 40%, #000 8%, transparent 0%),
            radial-gradient(ellipse at 65% 40%, #000 8%, transparent 0%),
            radial-gradient(ellipse at 50% 60%, #000 8%, transparent 0%)
          `,
          borderRadius: '100% / 70% 70% 0% 0%',
          position: 'relative',
          opacity: 0.9,
          mixBlendMode: 'exclusion',
          transform: 'rotateZ(-90deg)',
        }}
        animate={{
          x: ['30em', '-35em'],
        }}
        transition={{
          duration: ghostSpeed,
          ease: 'easeOut',
          repeat: Infinity,
        }}
      >
        {/* Ghost legs */}
        <div style={{ position: 'absolute', width: '20%', backgroundColor: ghostColor, height: '7vmin', left: 0, bottom: '-6vmin', borderRadius: '100% / 0% 0% 50% 50%' }} />
        <div style={{ position: 'absolute', width: '20%', backgroundColor: 'transparent', height: '4vmin', left: '20%', bottom: '-3vmin', borderRadius: '100% / 50% 50% 0% 0%' }} />
        <div style={{ position: 'absolute', width: '20%', backgroundColor: ghostColor, height: '4vmin', left: '40%', bottom: '-3.95vmin', borderRadius: '100% / 0% 0% 60% 60%' }} />
        <div style={{ position: 'absolute', width: '20%', backgroundColor: 'transparent', height: '4vmin', left: '60%', bottom: '-3vmin', borderRadius: '100% / 50% 50% 0% 0%' }} />
        <div style={{ position: 'absolute', width: '20%', backgroundColor: ghostColor, height: '3vmin', left: '80%', bottom: '-2.9vmin', borderRadius: '100% / 0% 0% 70% 70%' }} />
      </motion.div>
    </div>
  );
};

/**
 * CyberGhost - Cyberpunk-styled ghost with neon effects
 */
export const CyberGhost: React.FC<{
  text: string;
  className?: string;
  ghostSpeed?: number;
}> = ({ text, className = '', ghostSpeed = 4 }) => {
  return (
    <div
      className={`cyber-ghost-container ${className}`}
      style={{
        width: '100%',
        minHeight: '400px',
        display: 'grid',
        placeItems: 'center',
        overflow: 'hidden',
        background: 'linear-gradient(180deg, #0a0a1a 0%, #1a0a2e 100%)',
        position: 'relative',
        borderRadius: '1rem',
      }}
    >
      {/* Glowing text */}
      <h1
        style={{
          fontSize: 'clamp(3rem, 15vw, 12rem)',
          fontWeight: 900,
          fontFamily: "'Orbitron', monospace",
          color: 'transparent',
          WebkitTextStroke: '2px #00ffea',
          textShadow: '0 0 30px #00ffea44, 0 0 60px #00ffea22',
          position: 'absolute',
          userSelect: 'none',
        }}
      >
        {text}
      </h1>

      {/* Cyber Ghost */}
      <motion.div
        style={{
          width: '10vmin',
          height: '14vmin',
          background: 'linear-gradient(180deg, #00ffea 0%, #ff00ff 100%)',
          borderRadius: '100% / 70% 70% 0% 0%',
          position: 'relative',
          opacity: 0.8,
          mixBlendMode: 'screen',
          filter: 'blur(1px)',
          boxShadow: '0 0 30px #00ffea, 0 0 60px #ff00ff',
        }}
        animate={{
          x: ['40vw', '-40vw'],
          rotate: -90,
        }}
        transition={{
          duration: ghostSpeed,
          ease: [0.25, 0.1, 0.25, 1],
          repeat: Infinity,
        }}
      >
        {/* Ghost face */}
        <div style={{
          position: 'absolute',
          top: '30%',
          left: '25%',
          width: '15%',
          height: '20%',
          borderRadius: '50%',
          background: '#000',
          boxShadow: '2.5vmin 0 0 #000',
        }} />
        <div style={{
          position: 'absolute',
          top: '55%',
          left: '35%',
          width: '30%',
          height: '15%',
          borderRadius: '50%',
          background: '#000',
        }} />

        {/* Wavy bottom */}
        <svg
          style={{
            position: 'absolute',
            bottom: '-3vmin',
            left: 0,
            width: '100%',
            height: '4vmin',
          }}
          viewBox="0 0 100 40"
        >
          <path
            d="M0,20 Q12.5,40 25,20 T50,20 T75,20 T100,20 L100,40 L0,40 Z"
            fill="url(#ghostGradient)"
          />
          <defs>
            <linearGradient id="ghostGradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="#00ffea" />
              <stop offset="100%" stopColor="#ff00ff" />
            </linearGradient>
          </defs>
        </svg>
      </motion.div>

      {/* Scanlines */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          background: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.1) 2px, rgba(0,0,0,0.1) 4px)',
          pointerEvents: 'none',
        }}
      />
    </div>
  );
};

/**
 * GhostTextShowcase - Display ghost text variations
 */
export const GhostTextShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`ghost-text-showcase ${className}`} style={{ display: 'flex', flexDirection: 'column', gap: '2rem' }}>
      <GhostText text="GHOST" ghostSpeed={4} />
      <CyberGhost text="X3" ghostSpeed={5} />
      <GhostText text="BOO!" ghostSpeed={3} fontSize="30vmin" />
    </div>
  );
};

export default GhostText;
