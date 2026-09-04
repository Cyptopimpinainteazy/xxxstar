'use client';

import { motion } from 'framer-motion';
import React, { ReactNode } from 'react';

interface RainbowBorderProps {
  children: ReactNode;
  className?: string;
  borderWidth?: number;
  animated?: boolean;
  variant?: 'rainbow' | 'neon' | 'cyber' | 'sunset';
  rounded?: 'none' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'full';
}

export const RainbowBorder: React.FC<RainbowBorderProps> = ({
  children,
  className = '',
  borderWidth = 2,
  animated = true,
  variant = 'rainbow',
  rounded = 'xl',
}) => {
  const gradients = {
    rainbow: 'linear-gradient(90deg, #ff0080, #ff8c00, #40e0d0, #9400d3, #ff0080)',
    neon: 'linear-gradient(90deg, #00ff00, #00ffff, #ff00ff, #00ff00)',
    cyber: 'linear-gradient(90deg, #00d4ff, #7c3aed, #f472b6, #00d4ff)',
    sunset: 'linear-gradient(90deg, #f97316, #ec4899, #8b5cf6, #f97316)',
  };

  const roundedClasses = {
    none: 'rounded-none',
    sm: 'rounded-sm',
    md: 'rounded-md',
    lg: 'rounded-lg',
    xl: 'rounded-xl',
    '2xl': 'rounded-2xl',
    full: 'rounded-full',
  };

  return (
    <div className={`rainbow-border relative ${roundedClasses[rounded]} ${className}`}>
      {/* Animated Border */}
      <motion.div
        className={`absolute inset-0 ${roundedClasses[rounded]}`}
        style={{
          background: gradients[variant],
          backgroundSize: '400% 100%',
          padding: borderWidth,
          mask: 'linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)',
          WebkitMask: 'linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)',
          maskComposite: 'exclude',
          WebkitMaskComposite: 'xor',
        }}
        animate={animated ? {
          backgroundPosition: ['0% 0%', '100% 0%'],
        } : {}}
        transition={{
          duration: 3,
          repeat: Infinity,
          ease: 'linear',
        }}
      />

      {/* Content */}
      <div className="relative z-10">
        {children}
      </div>
    </div>
  );
};

// Glow Border Effect
interface GlowBorderProps {
  children: ReactNode;
  className?: string;
  color?: string;
  intensity?: 'low' | 'medium' | 'high';
  rounded?: string;
}

export const GlowBorder: React.FC<GlowBorderProps> = ({
  children,
  className = '',
  color = 'cyan',
  intensity = 'medium',
  rounded = 'rounded-xl',
}) => {
  const glowColors: Record<string, string> = {
    cyan: 'rgba(0, 255, 255, VAR)',
    purple: 'rgba(147, 51, 234, VAR)',
    pink: 'rgba(236, 72, 153, VAR)',
    green: 'rgba(34, 197, 94, VAR)',
    gold: 'rgba(234, 179, 8, VAR)',
  };

  const intensityValues = {
    low: { opacity: 0.3, blur: 15, spread: 5 },
    medium: { opacity: 0.5, blur: 25, spread: 10 },
    high: { opacity: 0.7, blur: 40, spread: 15 },
  };

  const config = intensityValues[intensity];
  const glowColor = glowColors[color] || glowColors.cyan;

  return (
    <motion.div
      className={`glow-border relative ${rounded} ${className}`}
      whileHover={{
        boxShadow: `0 0 ${config.blur * 1.5}px ${config.spread * 1.5}px ${glowColor.replace('VAR', String(config.opacity * 1.2))}`,
      }}
      style={{
        boxShadow: `0 0 ${config.blur}px ${config.spread}px ${glowColor.replace('VAR', String(config.opacity))}`,
      }}
      transition={{ duration: 0.3 }}
    >
      <div>{children}</div>
    </motion.div>
  );
};

// Pulsing Border
interface PulsingBorderProps {
  children: ReactNode;
  className?: string;
  color?: string;
  duration?: number;
  rounded?: string;
}

export const PulsingBorder: React.FC<PulsingBorderProps> = ({
  children,
  className = '',
  color = 'cyan',
  duration = 2,
  rounded = 'rounded-xl',
}) => {
  const borderColors: Record<string, string> = {
    cyan: 'border-cyan-500',
    purple: 'border-purple-500',
    pink: 'border-pink-500',
    green: 'border-green-500',
    gold: 'border-yellow-500',
  };

  return (
    <motion.div
      className={`pulsing-border relative border-2 ${borderColors[color] || borderColors.cyan} ${rounded} ${className}`}
      animate={{
        borderColor: [
          'rgba(0, 255, 255, 0.3)',
          'rgba(0, 255, 255, 1)',
          'rgba(0, 255, 255, 0.3)',
        ],
      }}
      transition={{
        duration,
        repeat: Infinity,
        ease: 'easeInOut',
      }}
    >
      <div>{children}</div>
    </motion.div>
  );
};

// Corner Accent Border
interface CornerAccentBorderProps {
  children: ReactNode;
  className?: string;
  color?: string;
  size?: number;
  thickness?: number;
}

export const CornerAccentBorder: React.FC<CornerAccentBorderProps> = ({
  children,
  className = '',
  color = 'cyan',
  size = 20,
  thickness = 2,
}) => {
  const colors: Record<string, string> = {
    cyan: 'border-cyan-500',
    purple: 'border-purple-500',
    pink: 'border-pink-500',
    gold: 'border-yellow-500',
  };

  const borderColor = colors[color] || colors.cyan;

  return (
    <div className={`corner-accent-border relative ${className}`}>
      {/* Top Left */}
      <div 
        className={`absolute top-0 left-0 border-t-${thickness} border-l-${thickness} ${borderColor}`}
        style={{ width: size, height: size }}
      />
      {/* Top Right */}
      <div 
        className={`absolute top-0 right-0 border-t-${thickness} border-r-${thickness} ${borderColor}`}
        style={{ width: size, height: size }}
      />
      {/* Bottom Left */}
      <div 
        className={`absolute bottom-0 left-0 border-b-${thickness} border-l-${thickness} ${borderColor}`}
        style={{ width: size, height: size }}
      />
      {/* Bottom Right */}
      <div 
        className={`absolute bottom-0 right-0 border-b-${thickness} border-r-${thickness} ${borderColor}`}
        style={{ width: size, height: size }}
      />

      {/* Content */}
      <div className="relative">
        {children}
      </div>
    </div>
  );
};
