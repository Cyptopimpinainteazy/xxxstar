'use client';

import { motion, useMotionValue, useSpring, useTransform } from 'framer-motion';
import React, { useRef, ReactNode } from 'react';

interface TiltCard3DProps {
  children: ReactNode;
  className?: string;
  intensity?: number;
  perspective?: number;
  glareEnabled?: boolean;
  borderGlow?: boolean;
}

export const TiltCard3D: React.FC<TiltCard3DProps> = ({
  children,
  className = '',
  intensity = 15,
  perspective = 1000,
  glareEnabled = true,
  borderGlow = true,
}) => {
  const cardRef = useRef<HTMLDivElement>(null);
  const mouseX = useMotionValue(0.5);
  const mouseY = useMotionValue(0.5);

  const rotateX = useSpring(
    useTransform(mouseY, [0, 1], [intensity, -intensity]),
    { stiffness: 150, damping: 20 }
  );
  const rotateY = useSpring(
    useTransform(mouseX, [0, 1], [-intensity, intensity]),
    { stiffness: 150, damping: 20 }
  );

  const glareX = useTransform(mouseX, [0, 1], ['0%', '100%']);
  const glareY = useTransform(mouseY, [0, 1], ['0%', '100%']);
  const glareOpacity = useTransform(
    mouseX,
    [0, 0.5, 1],
    [0.4, 0.1, 0.4]
  );

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!cardRef.current) return;
    const rect = cardRef.current.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    mouseX.set(x);
    mouseY.set(y);
  };

  const handleMouseLeave = () => {
    mouseX.set(0.5);
    mouseY.set(0.5);
  };

  return (
    <motion.div
      ref={cardRef}
      className={`tilt-card-3d relative overflow-hidden rounded-2xl ${className}`}
      style={{
        perspective,
        transformStyle: 'preserve-3d',
        rotateX,
        rotateY,
      }}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      {/* Card Content */}
      <div className="relative z-10" style={{ transform: 'translateZ(30px)' }}>
        {children}
      </div>

      {/* Glare Effect */}
      {glareEnabled && (
        <motion.div
          className="absolute inset-0 pointer-events-none"
          style={{
            background: `radial-gradient(circle at ${glareX} ${glareY}, rgba(255,255,255,0.3), transparent 60%)`,
            opacity: glareOpacity,
          }}
        />
      )}

      {/* Border Glow */}
      {borderGlow && (
        <motion.div
          className="absolute inset-0 rounded-2xl pointer-events-none"
          style={{
            boxShadow: 'inset 0 0 30px rgba(0, 255, 255, 0.1), 0 0 30px rgba(0, 255, 255, 0.1)',
          }}
          whileHover={{
            boxShadow: 'inset 0 0 50px rgba(0, 255, 255, 0.2), 0 0 60px rgba(0, 255, 255, 0.3)',
          }}
        />
      )}
    </motion.div>
  );
};

// Floating 3D Card with rotation animation
interface FloatingCard3DProps {
  children: ReactNode;
  className?: string;
  delay?: number;
}

export const FloatingCard3D: React.FC<FloatingCard3DProps> = ({
  children,
  className = '',
  delay = 0,
}) => {
  return (
    <motion.div
      className={`floating-card-3d ${className}`}
      initial={{ opacity: 0, y: 50 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      animate={{
        y: [0, -10, 0],
        rotateY: [0, 5, 0, -5, 0],
      }}
      transition={{
        delay,
        duration: 0.6,
        y: { duration: 4, repeat: Infinity, ease: 'easeInOut' },
        rotateY: { duration: 8, repeat: Infinity, ease: 'easeInOut' },
      }}
      style={{ perspective: 1000 }}
    >
      <div>{children}</div>
    </motion.div>
  );
};

// Perspective Container
interface PerspectiveContainerProps {
  children: ReactNode;
  className?: string;
  perspective?: number;
}

export const PerspectiveContainer: React.FC<PerspectiveContainerProps> = ({
  children,
  className = '',
  perspective = 1500,
}) => {
  return (
    <div
      className={`perspective-container ${className}`}
      style={{ perspective, transformStyle: 'preserve-3d' }}
    >
      {children}
    </div>
  );
};
