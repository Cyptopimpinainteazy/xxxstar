'use client';

import { motion } from 'framer-motion';
import React from 'react';

interface AnimatedSphereProps {
  size?: 'sm' | 'md' | 'lg' | 'xl';
  variant?: 'rainbow' | 'cyan' | 'purple' | 'gold' | 'neon';
  animated?: boolean;
  className?: string;
  pulsate?: boolean;
}

export const AnimatedSphere: React.FC<AnimatedSphereProps> = ({
  size = 'md',
  variant = 'rainbow',
  animated = true,
  pulsate = true,
  className = '',
}) => {
  const sizes = {
    sm: 'w-16 h-16',
    md: 'w-24 h-24',
    lg: 'w-32 h-32',
    xl: 'w-48 h-48',
  };

  const gradients = {
    rainbow: 'bg-gradient-conic from-red-500 via-yellow-500 via-green-500 via-cyan-500 via-blue-500 via-purple-500 to-red-500',
    cyan: 'bg-gradient-to-br from-cyan-300 via-cyan-500 to-teal-700',
    purple: 'bg-gradient-to-br from-purple-300 via-purple-500 to-indigo-700',
    gold: 'bg-gradient-to-br from-yellow-300 via-amber-500 to-orange-700',
    neon: 'bg-gradient-to-br from-green-400 via-cyan-400 to-purple-500',
  };

  // Fallback for rainbow (conic gradients need inline style)
  const rainbowStyle = variant === 'rainbow' ? {
    background: 'conic-gradient(from 0deg, #ff0080, #ff8c00, #40e0d0, #8a2be2, #ff0080)',
  } : {};

  return (
    <div className={`animated-sphere relative ${sizes[size]} ${className}`}>
      {/* Main Sphere */}
      <motion.div
        className={`w-full h-full rounded-full ${variant !== 'rainbow' ? gradients[variant] : ''} shadow-2xl relative overflow-hidden`}
        style={rainbowStyle}
        animate={animated ? {
          rotateY: [0, 360],
          rotateX: [0, 15, 0, -15, 0],
        } : {}}
        transition={{
          rotateY: { duration: 10, repeat: Infinity, ease: 'linear' },
          rotateX: { duration: 5, repeat: Infinity, ease: 'easeInOut' },
        }}
      >
        {/* Highlight */}
        <div className="absolute top-[10%] left-[15%] w-[30%] h-[30%] bg-white/50 rounded-full blur-md" />
        <div className="absolute top-[15%] left-[20%] w-[15%] h-[15%] bg-white/70 rounded-full blur-sm" />
        
        {/* Scanlines Effect */}
        <div 
          className="absolute inset-0 opacity-20"
          style={{
            background: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(255,255,255,0.1) 2px, rgba(255,255,255,0.1) 4px)',
          }}
        />
      </motion.div>

      {/* Glow Effect */}
      {pulsate && (
        <motion.div
          className={`absolute inset-0 rounded-full ${variant !== 'rainbow' ? gradients[variant] : ''} blur-xl`}
          style={variant === 'rainbow' ? {
            background: 'conic-gradient(from 0deg, #ff0080, #ff8c00, #40e0d0, #8a2be2, #ff0080)',
          } : {}}
          animate={{
            opacity: [0.3, 0.6, 0.3],
            scale: [1, 1.2, 1],
          }}
          transition={{
            duration: 2,
            repeat: Infinity,
            ease: 'easeInOut',
          }}
        />
      )}

      {/* Orbital Ring */}
      <motion.div
        className="absolute inset-[-20%] border-2 border-white/20 rounded-full"
        style={{ transform: 'rotateX(70deg)' }}
        animate={{
          rotateZ: [0, 360],
        }}
        transition={{
          duration: 15,
          repeat: Infinity,
          ease: 'linear',
        }}
      />
    </div>
  );
};

// Floating Spheres Background
interface FloatingSpheresProps {
  count?: number;
  className?: string;
}

export const FloatingSpheres: React.FC<FloatingSpheresProps> = ({
  count = 5,
  className = '',
}) => {
  const variants = ['cyan', 'purple', 'gold', 'neon', 'rainbow'] as const;
  const sizes = ['sm', 'md', 'lg'] as const;

  return (
    <div className={`floating-spheres absolute inset-0 overflow-hidden pointer-events-none ${className}`}>
      {Array.from({ length: count }).map((_, i) => (
        <motion.div
          key={i}
          className="absolute"
          style={{
            left: `${Math.random() * 80 + 10}%`,
            top: `${Math.random() * 80 + 10}%`,
          }}
          animate={{
            x: [0, Math.random() * 100 - 50, 0],
            y: [0, Math.random() * 100 - 50, 0],
          }}
          transition={{
            duration: 10 + Math.random() * 10,
            repeat: Infinity,
            ease: 'easeInOut',
            delay: i * 0.5,
          }}
        >
          <AnimatedSphere
            size={sizes[i % sizes.length]}
            variant={variants[i % variants.length]}
            pulsate={i % 2 === 0}
          />
        </motion.div>
      ))}
    </div>
  );
};

// Data Orb with Statistics
interface DataOrbProps {
  value: string | number;
  label: string;
  variant?: 'cyan' | 'purple' | 'gold';
  className?: string;
}

export const DataOrb: React.FC<DataOrbProps> = ({
  value,
  label,
  variant = 'cyan',
  className = '',
}) => {
  const colors = {
    cyan: 'from-cyan-400 to-cyan-600',
    purple: 'from-purple-400 to-purple-600',
    gold: 'from-amber-400 to-amber-600',
  };

  return (
    <motion.div
      className={`data-orb relative ${className}`}
      initial={{ opacity: 0, scale: 0 }}
      whileInView={{ opacity: 1, scale: 1 }}
      viewport={{ once: true }}
      transition={{ type: 'spring', stiffness: 200, damping: 20 }}
    >
      <div className={`w-32 h-32 rounded-full bg-gradient-to-br ${colors[variant]} flex flex-col items-center justify-center shadow-2xl relative`}>
        <span className="text-2xl font-bold text-white">{value}</span>
        <span className="text-xs text-white/80 uppercase tracking-wider">{label}</span>
        
        {/* Shine */}
        <div className="absolute top-3 left-5 w-6 h-6 bg-white/40 rounded-full blur-sm" />
      </div>
      
      {/* Glow */}
      <div className={`absolute inset-0 rounded-full bg-gradient-to-br ${colors[variant]} blur-xl opacity-40`} />
    </motion.div>
  );
};
