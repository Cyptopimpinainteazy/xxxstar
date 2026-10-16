'use client';

import { motion, useMotionValue, useSpring, useTransform } from 'framer-motion';
import React, { useRef, useState } from 'react';

// ============================================
// SCOTCH VHS CASSETTE CARD
// Inspired by vintage cassette tape design
// ============================================

interface ScotchCassetteCardProps {
  title: string;
  subtitle?: string;
  duration?: string;
  catalogNumber?: string;
  features?: string[];
  sphereColor?: 'rainbow' | 'gold' | 'chrome' | 'holographic';
  onPlay?: () => void;
  className?: string;
}

export const ScotchCassetteCard: React.FC<ScotchCassetteCardProps> = ({
  title,
  subtitle,
  duration = '120',
  catalogNumber = 'QTM-2060',
  features = [],
  sphereColor = 'rainbow',
  onPlay,
  className = '',
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);
  
  const mouseX = useMotionValue(0);
  const mouseY = useMotionValue(0);
  
  const rotateX = useSpring(useTransform(mouseY, [-0.5, 0.5], [5, -5]), { stiffness: 100, damping: 20 });
  const rotateY = useSpring(useTransform(mouseX, [-0.5, 0.5], [-5, 5]), { stiffness: 100, damping: 20 });

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!cardRef.current) return;
    const rect = cardRef.current.getBoundingClientRect();
    mouseX.set((e.clientX - rect.left) / rect.width - 0.5);
    mouseY.set((e.clientY - rect.top) / rect.height - 0.5);
  };

  const sphereGradients = {
    rainbow: 'conic-gradient(from 0deg, #ff0080, #ff8c00, #40e0d0, #9400d3, #ff0080)',
    gold: 'linear-gradient(135deg, #ffd700, #ffaa00, #ff8c00, #ffd700)',
    chrome: 'linear-gradient(135deg, #e0e0e0, #ffffff, #a0a0a0, #e0e0e0)',
    holographic: 'linear-gradient(135deg, #ff00ff, #00ffff, #ffff00, #ff00ff)',
  };

  return (
    <motion.div
      ref={cardRef}
      className={`scotch-cassette-card relative overflow-hidden rounded-2xl ${className}`}
      style={{
        perspective: 1000,
        rotateX,
        rotateY,
      }}
      onMouseMove={handleMouseMove}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => {
        setIsHovered(false);
        mouseX.set(0);
        mouseY.set(0);
      }}
      initial={{ opacity: 0, y: 40 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ duration: 0.6 }}
    >
      {/* Main Card Body */}
      <div className="relative bg-gradient-to-br from-[#1a1a2e] via-[#16162a] to-[#0d0d1a] border border-white/10 rounded-2xl p-8 min-h-[450px]">
        
        {/* Noise Texture */}
        <div 
          className="absolute inset-0 opacity-20 mix-blend-overlay pointer-events-none rounded-2xl"
          style={{
            backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E")`,
          }}
        />

        {/* VHS Scanlines */}
        <div 
          className="absolute inset-0 pointer-events-none opacity-20 rounded-2xl"
          style={{
            background: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.3) 2px, rgba(0,0,0,0.3) 4px)',
          }}
        />

        {/* Diagonal Stripes */}
        <div 
          className="absolute inset-0 opacity-5 pointer-events-none rounded-2xl"
          style={{
            background: 'repeating-linear-gradient(45deg, transparent, transparent 10px, rgba(255,255,255,0.1) 10px, rgba(255,255,255,0.1) 20px)',
          }}
        />

        {/* Top Section - Brand Label */}
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="w-3 h-3 bg-red-500 rounded-full animate-pulse shadow-lg shadow-red-500/50" />
            <span className="text-xs font-mono uppercase tracking-[0.3em] text-white/50">
              ATLAS SPHERE
            </span>
          </div>
          <div className="text-right">
            <div className="text-[10px] font-mono text-white/30 uppercase tracking-widest">
              EST. 2060
            </div>
            <div className="text-xs font-mono text-cyan-400/60">
              {catalogNumber}
            </div>
          </div>
        </div>

        {/* Center Section - Sphere & Title */}
        <div className="flex items-start gap-8 mb-8">
          {/* Animated Sphere */}
          <motion.div
            className="relative flex-shrink-0"
            animate={{
              rotateY: isHovered ? 360 : 0,
            }}
            transition={{
              duration: 8,
              repeat: Infinity,
              ease: 'linear',
            }}
          >
            {/* Main Sphere */}
            <div 
              className="w-28 h-28 rounded-full shadow-2xl relative"
              style={{ background: sphereGradients[sphereColor] }}
            >
              {/* Highlight */}
              <div className="absolute top-3 left-5 w-8 h-8 bg-white/50 rounded-full blur-md" />
              <div className="absolute top-5 left-7 w-3 h-3 bg-white/80 rounded-full" />
              
              {/* Scanline on sphere */}
              <div 
                className="absolute inset-0 rounded-full opacity-30"
                style={{
                  background: 'repeating-linear-gradient(0deg, transparent, transparent 3px, rgba(255,255,255,0.1) 3px, rgba(255,255,255,0.1) 6px)',
                }}
              />
            </div>
            
            {/* Sphere Glow */}
            <motion.div
              className="absolute inset-[-30%] rounded-full blur-2xl"
              style={{ background: sphereGradients[sphereColor] }}
              animate={{
                opacity: isHovered ? 0.5 : 0.25,
                scale: isHovered ? 1.1 : 1,
              }}
              transition={{ duration: 0.4 }}
            />
          </motion.div>

          {/* Title & Subtitle */}
          <div className="flex-grow pt-2">
            {subtitle && (
              <motion.p
                className="text-xs font-semibold uppercase tracking-[0.25em] text-cyan-400/70 mb-2"
                animate={{ letterSpacing: isHovered ? '0.3em' : '0.25em' }}
                transition={{ duration: 0.3 }}
              >
                {subtitle}
              </motion.p>
            )}
            <motion.h3
              className="text-4xl font-bold text-white leading-tight mb-4"
              style={{ fontFamily: "'Oswald', sans-serif" }}
              animate={{ y: isHovered ? -2 : 0 }}
              transition={{ duration: 0.3 }}
            >
              {title}
            </motion.h3>
            
            {/* Duration Badge */}
            <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-white/5 border border-white/10">
              <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
              <span className="text-xs font-mono text-white/60">{duration} MIN</span>
            </div>
          </div>
        </div>

        {/* Feature List */}
        {features.length > 0 && (
          <div className="mb-8">
            <ul className="space-y-2">
              {features.map((feature, index) => (
                <motion.li
                  key={index}
                  className="flex items-center gap-3 text-sm text-white/60"
                  initial={{ opacity: 0, x: -10 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.1 }}
                >
                  <span className="w-1.5 h-1.5 rounded-full bg-gradient-to-r from-cyan-400 to-purple-400" />
                  {feature}
                </motion.li>
              ))}
            </ul>
          </div>
        )}

        {/* Bottom Section - CTA */}
        <div className="flex items-center justify-between pt-6 border-t border-white/10">
          <motion.button
            onClick={onPlay}
            className="flex items-center gap-3 px-6 py-3 rounded-full bg-gradient-to-r from-cyan-500 to-purple-500 text-white font-bold text-sm uppercase tracking-wider shadow-lg shadow-cyan-500/30"
            whileHover={{ scale: 1.05, boxShadow: '0 10px 40px rgba(0, 255, 255, 0.4)' }}
            whileTap={{ scale: 0.95 }}
          >
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
              <path d="M8 5v14l11-7z" />
            </svg>
            Play Now
          </motion.button>

          <motion.div
            className="flex items-center gap-2 text-white/40"
            animate={{ x: isHovered ? 5 : 0, opacity: isHovered ? 1 : 0.6 }}
            transition={{ duration: 0.3 }}
          >
            <span className="text-xs uppercase tracking-wider">Explore</span>
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 8l4 4m0 0l-4 4m4-4H3" />
            </svg>
          </motion.div>
        </div>

        {/* Tape Reels (Decorative) */}
        <div className="absolute bottom-4 right-4 flex gap-3 opacity-20">
          <motion.div
            className="w-8 h-8 border-2 border-white/30 rounded-full"
            animate={{ rotate: isHovered ? 360 : 0 }}
            transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}
          >
            <div className="absolute inset-2 bg-white/10 rounded-full" />
          </motion.div>
          <motion.div
            className="w-8 h-8 border-2 border-white/30 rounded-full"
            animate={{ rotate: isHovered ? -360 : 0 }}
            transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}
          >
            <div className="absolute inset-2 bg-white/10 rounded-full" />
          </motion.div>
        </div>
      </div>

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

      {/* GIF Overlay Effect */}
      <motion.div
        className="absolute inset-0 rounded-2xl pointer-events-none mix-blend-overlay"
        style={{
          backgroundImage: 'url("/media/gallery/static-noise.gif")',
          backgroundSize: 'cover',
        }}
        animate={{ opacity: isHovered ? 0.08 : 0 }}
        transition={{ duration: 0.3 }}
      />
    </motion.div>
  );
};

// ============================================
// SCOTCH CASSETTE SHOWCASE GRID
// ============================================

interface ScotchShowcaseProps {
  title?: string;
  className?: string;
}

export const ScotchCassetteShowcase: React.FC<ScotchShowcaseProps> = ({
  title = 'QUANTUM MODULES',
  className = '',
}) => {
  const cassettes = [
    {
      title: 'CROSS-CHAIN BRIDGE',
      subtitle: 'QUANTUM MODULE',
      duration: '∞',
      catalogNumber: 'XCB-001',
      sphereColor: 'rainbow' as const,
      features: ['Atomic swaps across 50+ chains', 'Sub-second finality', 'Zero slippage guarantee', 'MEV protection'],
    },
    {
      title: 'NEURAL CONSENSUS',
      subtitle: 'AI VALIDATOR',
      duration: '24/7',
      catalogNumber: 'NCS-002',
      sphereColor: 'holographic' as const,
      features: ['Self-optimizing validation', 'Predictive block production', 'Adaptive gas pricing', 'Threat detection'],
    },
    {
      title: 'QUANTUM VAULT',
      subtitle: 'SECURE STORAGE',
      duration: '256',
      catalogNumber: 'QVT-003',
      sphereColor: 'gold' as const,
      features: ['Post-quantum encryption', 'Multi-sig support', 'Hardware wallet integration', 'Social recovery'],
    },
    {
      title: 'DEX AGGREGATOR',
      subtitle: 'LIQUIDITY HUB',
      duration: '1000+',
      catalogNumber: 'DEX-004',
      sphereColor: 'chrome' as const,
      features: ['Best price routing', 'Concentrated liquidity', 'Limit orders', 'Portfolio tracking'],
    },
  ];

  return (
    <section className={`py-20 px-4 ${className}`}>
      <div className="max-w-7xl mx-auto">
        {/* Section Header */}
        <motion.div
          className="text-center mb-16"
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
        >
          <h2 
            className="text-5xl md:text-6xl font-bold text-white mb-4"
            style={{ fontFamily: "'Oswald', sans-serif" }}
          >
            <span className="bg-gradient-to-r from-yellow-400 via-pink-500 to-purple-500 bg-clip-text text-transparent">
              {title}
            </span>
          </h2>
          <p className="text-lg text-white/50 max-w-2xl mx-auto">
            Experience the next generation of blockchain technology with our quantum-powered modules
          </p>
        </motion.div>

        {/* Cassette Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {cassettes.map((cassette, index) => (
            <motion.div
              key={cassette.catalogNumber}
              initial={{ opacity: 0, y: 30 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: index * 0.15, duration: 0.6 }}
            >
              <ScotchCassetteCard
                {...cassette}
                onPlay={() => console.log(`Playing ${cassette.title}`)}
              />
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
};

export default ScotchCassetteShowcase;
