'use client';

import { motion, useMotionValue, useTransform, useSpring } from 'framer-motion';
import React, { useRef, useState } from 'react';

interface VHSRetroCardProps {
  title: string;
  subtitle?: string;
  description?: string;
  features?: string[];
  gifSrc?: string;
  sphereColor?: 'cyan' | 'purple' | 'gold' | 'rainbow';
  variant?: 'dark' | 'gradient' | 'vintage';
  href?: string;
  className?: string;
}

export const VHSRetroCard: React.FC<VHSRetroCardProps> = ({
  title,
  subtitle,
  description,
  features = [],
  gifSrc,
  sphereColor = 'rainbow',
  variant = 'dark',
  href,
  className = '',
}) => {
  const [isHovered, setIsHovered] = useState(false);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const cardRef = useRef<any>(null);
  const mouseX = useMotionValue(0);
  const mouseY = useMotionValue(0);

  const rotateX = useSpring(useTransform(mouseY, [-0.5, 0.5], [8, -8]), { stiffness: 150, damping: 20 });
  const rotateY = useSpring(useTransform(mouseX, [-0.5, 0.5], [-8, 8]), { stiffness: 150, damping: 20 });

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!cardRef.current) return;
    const rect = cardRef.current.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width - 0.5;
    const y = (e.clientY - rect.top) / rect.height - 0.5;
    mouseX.set(x);
    mouseY.set(y);
  };

  const handleMouseLeave = () => {
    mouseX.set(0);
    mouseY.set(0);
    setIsHovered(false);
  };

  const sphereGradients = {
    cyan: 'from-cyan-400 via-cyan-600 to-teal-800',
    purple: 'from-purple-400 via-purple-600 to-indigo-800',
    gold: 'from-yellow-400 via-orange-500 to-red-600',
    rainbow: 'from-red-500 via-yellow-500 via-green-500 via-cyan-500 to-purple-500',
  };

  const variantStyles = {
    dark: 'bg-gradient-to-br from-gray-900/95 via-gray-800/95 to-black/95',
    gradient: 'bg-gradient-to-br from-purple-900/95 via-indigo-900/95 to-cyan-900/95',
    vintage: 'bg-gradient-to-br from-amber-900/95 via-orange-900/95 to-red-950/95',
  };

  const CardWrapper = href ? motion.a : motion.div;

  return (
    <CardWrapper
      ref={cardRef}
      href={href}
      className={`vhs-retro-card group relative block overflow-hidden rounded-2xl border border-white/10 backdrop-blur-sm ${variantStyles[variant]} ${className}`}
      style={{
        perspective: 1000,
        transformStyle: 'preserve-3d',
        rotateX,
        rotateY,
      }}
      onMouseMove={handleMouseMove}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={handleMouseLeave}
      initial={{ opacity: 0, y: 40 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ duration: 0.6, ease: 'easeOut' }}
    >
      {/* Noise Texture Overlay */}
      <div 
        className="absolute inset-0 opacity-20 mix-blend-overlay pointer-events-none"
        style={{
          backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E")`,
        }}
      />

      {/* VHS Scanlines */}
      <div 
        className="absolute inset-0 pointer-events-none opacity-30"
        style={{
          background: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.3) 2px, rgba(0,0,0,0.3) 4px)',
        }}
      />

      {/* Diagonal Stripes Background */}
      <div 
        className="absolute inset-0 opacity-5 pointer-events-none"
        style={{
          background: 'repeating-linear-gradient(45deg, transparent, transparent 10px, rgba(255,255,255,0.1) 10px, rgba(255,255,255,0.1) 20px)',
        }}
      />

      <div className="relative p-8 flex flex-col min-h-[400px]">
        {/* Top Section - Sphere and Branding */}
        <div className="flex items-start justify-between mb-6">
          {/* Animated Sphere */}
          <motion.div
            className="relative w-24 h-24"
            animate={{ 
              rotateY: isHovered ? 360 : 0,
              scale: isHovered ? 1.1 : 1,
            }}
            transition={{ 
              rotateY: { duration: 8, repeat: Infinity, ease: 'linear' },
              scale: { duration: 0.4 },
            }}
          >
            <div className={`w-full h-full rounded-full bg-gradient-to-br ${sphereGradients[sphereColor]} shadow-2xl`}>
              {/* Sphere Shine */}
              <div className="absolute top-2 left-4 w-4 h-4 bg-white/60 rounded-full blur-sm" />
              <div className="absolute top-4 left-6 w-2 h-2 bg-white/80 rounded-full" />
            </div>
            {/* Sphere Glow */}
            <motion.div
              className={`absolute inset-0 rounded-full bg-gradient-to-br ${sphereGradients[sphereColor]} opacity-50 blur-xl`}
              animate={{ opacity: isHovered ? 0.7 : 0.3 }}
              transition={{ duration: 0.4 }}
            />
          </motion.div>

          {/* Retro Badge */}
          <div className="flex flex-col items-end">
            <span 
              className="text-xs font-bold uppercase tracking-[0.3em] text-white/60"
              style={{ fontFamily: "'Courier New', monospace" }}
            >
              X3 CHAIN
            </span>
            <span className="text-[10px] text-white/40 mt-1">EST. 2060</span>
          </div>
        </div>

        {/* Middle Section - Title & Description */}
        <div className="flex-grow">
          {subtitle && (
            <motion.p
              className="text-xs font-semibold uppercase tracking-[0.2em] text-cyan-400/80 mb-2"
              animate={{ letterSpacing: isHovered ? '0.3em' : '0.2em' }}
              transition={{ duration: 0.3 }}
            >
              {subtitle}
            </motion.p>
          )}

          <motion.h3
            className="text-3xl md:text-4xl font-bold text-white mb-4 leading-tight"
            style={{ fontFamily: "'Oswald', sans-serif" }}
            animate={{ y: isHovered ? -4 : 0 }}
            transition={{ duration: 0.3 }}
          >
            {title}
          </motion.h3>

          {description && (
            <p className="text-sm text-white/60 leading-relaxed mb-6 line-clamp-3">
              {description}
            </p>
          )}

          {/* Feature List */}
          {features.length > 0 && (
            <ul className="space-y-2">
              {features.map((feature, index) => (
                <motion.li
                  key={index}
                  className="flex items-center gap-3 text-sm text-white/70"
                  initial={{ opacity: 0, x: -10 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.1 }}
                >
                  <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
                  {feature}
                </motion.li>
              ))}
            </ul>
          )}
        </div>

        {/* Bottom Section - CTA */}
        <div className="mt-6 pt-6 border-t border-white/10 flex items-center justify-between">
          <motion.button
            className="px-6 py-2 rounded-full bg-gradient-to-r from-cyan-500 to-purple-500 text-white text-sm font-bold uppercase tracking-wider"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            Learn More
          </motion.button>

          <motion.div
            className="flex items-center gap-2 text-white/40"
            animate={{ x: isHovered ? 5 : 0 }}
            transition={{ duration: 0.3 }}
          >
            <span className="text-xs uppercase tracking-wider">Explore</span>
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 8l4 4m0 0l-4 4m4-4H3" />
            </svg>
          </motion.div>
        </div>
      </div>

      {/* GIF Overlay on Hover */}
      {gifSrc && (
        <motion.div
          className="absolute inset-0 pointer-events-none"
          initial={{ opacity: 0 }}
          animate={{ opacity: isHovered ? 0.15 : 0 }}
          transition={{ duration: 0.3 }}
        >
          <img src={gifSrc} alt="" className="w-full h-full object-cover" />
        </motion.div>
      )}

      {/* VHS Glitch Effect on Hover */}
      <motion.div
        className="absolute inset-0 pointer-events-none"
        animate={{ 
          opacity: isHovered ? [0, 0.3, 0, 0.2, 0] : 0,
          x: isHovered ? [0, -2, 2, -1, 0] : 0,
        }}
        transition={{ duration: 0.2, repeat: isHovered ? Infinity : 0, repeatDelay: 2 }}
      >
        <div className="absolute inset-0 bg-red-500/20 mix-blend-multiply" style={{ clipPath: 'inset(45% 0 50% 0)' }} />
        <div className="absolute inset-0 bg-cyan-500/20 mix-blend-multiply" style={{ clipPath: 'inset(30% 0 60% 0)' }} />
      </motion.div>

      {/* Rainbow Border on Hover */}
      <motion.div
        className="absolute inset-0 rounded-2xl pointer-events-none"
        style={{
          background: 'linear-gradient(90deg, #ff0080, #ff8c00, #40e0d0, #9400d3, #ff0080)',
          backgroundSize: '400% 100%',
          padding: '2px',
          mask: 'linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)',
          WebkitMask: 'linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)',
          maskComposite: 'exclude',
          WebkitMaskComposite: 'xor',
        }}
        animate={{ 
          backgroundPosition: isHovered ? ['0% 0%', '100% 0%'] : '0% 0%',
          opacity: isHovered ? 1 : 0,
        }}
        transition={{ 
          backgroundPosition: { duration: 2, repeat: Infinity, ease: 'linear' },
          opacity: { duration: 0.3 },
        }}
      />
    </CardWrapper>
  );
};

// Grid for VHS cards
interface VHSRetroGridProps {
  children: React.ReactNode;
  columns?: 1 | 2 | 3;
  className?: string;
}

export const VHSRetroGrid: React.FC<VHSRetroGridProps> = ({
  children,
  columns = 2,
  className = '',
}) => {
  const gridCols = {
    1: 'md:grid-cols-1',
    2: 'md:grid-cols-2',
    3: 'md:grid-cols-3',
  };

  return (
    <div className={`grid grid-cols-1 ${gridCols[columns]} gap-8 ${className}`}>
      {children}
    </div>
  );
};
