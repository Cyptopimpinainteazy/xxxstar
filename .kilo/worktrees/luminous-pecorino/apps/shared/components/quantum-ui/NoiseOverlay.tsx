'use client';

import { motion } from 'framer-motion';
import React, { useEffect, useState } from 'react';

interface NoiseOverlayProps {
  opacity?: number;
  animate?: boolean;
  className?: string;
}

export const NoiseOverlay: React.FC<NoiseOverlayProps> = ({
  opacity = 0.15,
  animate = true,
  className = '',
}) => {
  const [seed, setSeed] = useState(0);

  useEffect(() => {
    if (!animate) return;
    const interval = setInterval(() => {
      setSeed(prev => (prev + 1) % 100);
    }, 100);
    return () => clearInterval(interval);
  }, [animate]);

  return (
    <div
      className={`noise-overlay absolute inset-0 pointer-events-none mix-blend-overlay ${className}`}
      style={{ opacity }}
    >
      <svg className="w-full h-full" xmlns="http://www.w3.org/2000/svg">
        <filter id={`noise-${seed}`}>
          <feTurbulence
            type="fractalNoise"
            baseFrequency="0.8"
            numOctaves="4"
            seed={seed}
            stitchTiles="stitch"
          />
          <feColorMatrix type="saturate" values="0" />
        </filter>
        <rect width="100%" height="100%" filter={`url(#noise-${seed})`} />
      </svg>
    </div>
  );
};

// Static Noise Background (lighter weight)
interface StaticNoiseProps {
  opacity?: number;
  className?: string;
}

export const StaticNoise: React.FC<StaticNoiseProps> = ({
  opacity = 0.1,
  className = '',
}) => {
  return (
    <div
      className={`static-noise absolute inset-0 pointer-events-none ${className}`}
      style={{
        opacity,
        backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E")`,
      }}
    />
  );
};

// VHS Scanlines
interface ScanlinesProps {
  spacing?: number;
  opacity?: number;
  className?: string;
}

export const Scanlines: React.FC<ScanlinesProps> = ({
  spacing = 4,
  opacity = 0.3,
  className = '',
}) => {
  return (
    <div
      className={`scanlines absolute inset-0 pointer-events-none ${className}`}
      style={{
        background: `repeating-linear-gradient(
          0deg,
          transparent,
          transparent ${spacing / 2}px,
          rgba(0, 0, 0, ${opacity}) ${spacing / 2}px,
          rgba(0, 0, 0, ${opacity}) ${spacing}px
        )`,
      }}
    />
  );
};

// CRT Screen Effect (combines multiple effects)
interface CRTEffectProps {
  className?: string;
  vignetteIntensity?: number;
  scanlineOpacity?: number;
  noiseOpacity?: number;
  flickerEnabled?: boolean;
}

export const CRTEffect: React.FC<CRTEffectProps> = ({
  className = '',
  vignetteIntensity = 0.4,
  scanlineOpacity = 0.15,
  noiseOpacity = 0.1,
  flickerEnabled = true,
}) => {
  return (
    <div className={`crt-effect absolute inset-0 pointer-events-none overflow-hidden ${className}`}>
      {/* Scanlines */}
      <Scanlines opacity={scanlineOpacity} />

      {/* Noise */}
      <StaticNoise opacity={noiseOpacity} />

      {/* Vignette */}
      <div
        className="absolute inset-0"
        style={{
          background: `radial-gradient(ellipse at center, transparent 40%, rgba(0,0,0,${vignetteIntensity}) 100%)`,
        }}
      />

      {/* Flicker */}
      {flickerEnabled && (
        <motion.div
          className="absolute inset-0 bg-black"
          animate={{
            opacity: [0, 0.03, 0, 0.02, 0, 0],
          }}
          transition={{
            duration: 0.5,
            repeat: Infinity,
            repeatDelay: 2,
          }}
        />
      )}

      {/* Color Aberration Lines */}
      <motion.div
        className="absolute inset-0 mix-blend-screen"
        animate={{
          opacity: [0, 0.1, 0],
        }}
        transition={{
          duration: 0.1,
          repeat: Infinity,
          repeatDelay: 3,
        }}
      >
        <div 
          className="h-[2px] bg-red-500/50 absolute"
          style={{ top: '30%', left: 0, right: 0 }}
        />
        <div 
          className="h-[2px] bg-cyan-500/50 absolute"
          style={{ top: '70%', left: 0, right: 0 }}
        />
      </motion.div>
    </div>
  );
};

// Diagonal Stripes Background
interface DiagonalStripesProps {
  color?: string;
  spacing?: number;
  opacity?: number;
  angle?: number;
  className?: string;
}

export const DiagonalStripes: React.FC<DiagonalStripesProps> = ({
  color = 'white',
  spacing = 20,
  opacity = 0.05,
  angle = 45,
  className = '',
}) => {
  return (
    <div
      className={`diagonal-stripes absolute inset-0 pointer-events-none ${className}`}
      style={{
        background: `repeating-linear-gradient(
          ${angle}deg,
          transparent,
          transparent ${spacing / 2}px,
          rgba(255, 255, 255, ${opacity}) ${spacing / 2}px,
          rgba(255, 255, 255, ${opacity}) ${spacing}px
        )`,
      }}
    />
  );
};
