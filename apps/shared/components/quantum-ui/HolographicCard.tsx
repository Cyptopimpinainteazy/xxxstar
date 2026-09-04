'use client';

import React, { useState, useRef, useEffect } from 'react';
import { motion, useMotionValue, useTransform, useSpring } from 'framer-motion';

interface HolographicCardProps {
  title: string;
  subtitle?: string;
  image?: string;
  rarity?: 'common' | 'rare' | 'epic' | 'legendary' | 'mythic';
  stats?: Array<{ label: string; value: string | number }>;
  className?: string;
  children?: React.ReactNode;
}

/**
 * HolographicCard - Pokemon/Trading card style holographic effect
 * 3D tilt with rainbow holographic shimmer, glitter, and parallax layers
 */
export const HolographicCard: React.FC<HolographicCardProps> = ({
  title,
  subtitle,
  image,
  rarity = 'legendary',
  stats = [],
  className = '',
  children,
}) => {
  const cardRef = useRef<HTMLDivElement>(null);
  const [isHovered, setIsHovered] = useState(false);
  
  const mouseX = useMotionValue(0);
  const mouseY = useMotionValue(0);
  
  const rotateX = useSpring(useTransform(mouseY, [-0.5, 0.5], [15, -15]), { stiffness: 300, damping: 30 });
  const rotateY = useSpring(useTransform(mouseX, [-0.5, 0.5], [-15, 15]), { stiffness: 300, damping: 30 });
  
  const glareX = useTransform(mouseX, [-0.5, 0.5], [0, 100]);
  const glareY = useTransform(mouseY, [-0.5, 0.5], [0, 100]);
  
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

  const rarityColors = {
    common: { border: '#888', glow: 'rgba(136,136,136,0.5)', gradient: 'linear-gradient(135deg, #666, #999)' },
    rare: { border: '#4a9eff', glow: 'rgba(74,158,255,0.5)', gradient: 'linear-gradient(135deg, #0066cc, #4a9eff)' },
    epic: { border: '#a855f7', glow: 'rgba(168,85,247,0.5)', gradient: 'linear-gradient(135deg, #7c3aed, #a855f7)' },
    legendary: { border: '#fbbf24', glow: 'rgba(251,191,36,0.5)', gradient: 'linear-gradient(135deg, #f59e0b, #fbbf24)' },
    mythic: { border: '#ff00ff', glow: 'rgba(255,0,255,0.5)', gradient: 'linear-gradient(135deg, #ff00ff, #00ffff, #ff00ff)' },
  };

  const colors = rarityColors[rarity];

  return (
    <motion.div
      ref={cardRef}
      className={`holographic-card ${className}`}
      onMouseMove={handleMouseMove}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={handleMouseLeave}
      style={{
        width: '280px',
        height: '400px',
        perspective: '1000px',
        cursor: 'pointer',
      }}
    >
      <motion.div
        className="card-inner"
        style={{
          width: '100%',
          height: '100%',
          borderRadius: '16px',
          background: 'linear-gradient(145deg, #1a1a2e, #16213e)',
          border: `3px solid ${colors.border}`,
          boxShadow: isHovered 
            ? `0 25px 50px rgba(0,0,0,0.5), 0 0 30px ${colors.glow}, inset 0 0 30px rgba(255,255,255,0.1)`
            : '0 10px 30px rgba(0,0,0,0.3)',
          rotateX,
          rotateY,
          transformStyle: 'preserve-3d',
          position: 'relative',
          overflow: 'hidden',
        }}
      >
        {/* Holographic shimmer overlay */}
        <motion.div
          className="holographic-shimmer"
          style={{
            position: 'absolute',
            inset: 0,
            background: `
              linear-gradient(
                125deg,
                transparent 0%,
                rgba(255,0,0,0.1) 10%,
                rgba(255,127,0,0.1) 20%,
                rgba(255,255,0,0.1) 30%,
                rgba(0,255,0,0.1) 40%,
                rgba(0,0,255,0.1) 50%,
                rgba(75,0,130,0.1) 60%,
                rgba(148,0,211,0.1) 70%,
                transparent 100%
              )
            `,
            backgroundSize: '200% 200%',
            opacity: isHovered ? 0.8 : 0.3,
            mixBlendMode: 'overlay',
            pointerEvents: 'none',
          }}
          animate={{
            backgroundPosition: isHovered ? ['0% 0%', '100% 100%'] : '0% 0%',
          }}
          transition={{
            duration: 3,
            repeat: isHovered ? Infinity : 0,
            ease: 'linear',
          }}
        />

        {/* Glare effect */}
        <motion.div
          className="glare"
          style={{
            position: 'absolute',
            inset: '-50%',
            background: `radial-gradient(circle at ${glareX}% ${glareY}%, rgba(255,255,255,0.3) 0%, transparent 50%)`,
            opacity: isHovered ? 1 : 0,
            pointerEvents: 'none',
            transition: 'opacity 0.3s',
          }}
        />

        {/* Sparkle particles */}
        {isHovered && [...Array(20)].map((_, i) => (
          <motion.div
            key={i}
            style={{
              position: 'absolute',
              width: '4px',
              height: '4px',
              borderRadius: '50%',
              background: 'white',
              left: `${Math.random() * 100}%`,
              top: `${Math.random() * 100}%`,
              boxShadow: '0 0 6px white',
            }}
            animate={{
              opacity: [0, 1, 0],
              scale: [0, 1, 0],
            }}
            transition={{
              duration: 1 + Math.random(),
              repeat: Infinity,
              delay: Math.random() * 2,
            }}
          />
        ))}

        {/* Card content */}
        <div style={{ position: 'relative', zIndex: 1, padding: '16px', height: '100%', display: 'flex', flexDirection: 'column' }}>
          {/* Rarity badge */}
          <div style={{
            position: 'absolute',
            top: '10px',
            right: '10px',
            padding: '4px 12px',
            background: colors.gradient,
            borderRadius: '20px',
            fontSize: '10px',
            fontWeight: 'bold',
            textTransform: 'uppercase',
            letterSpacing: '1px',
            color: rarity === 'legendary' || rarity === 'common' ? '#000' : '#fff',
          }}>
            {rarity}
          </div>

          {/* Image area */}
          <div style={{
            flex: 1,
            borderRadius: '8px',
            background: image ? `url(${image}) center/cover` : 'linear-gradient(135deg, #0a0a0a, #1a1a1a)',
            marginBottom: '12px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            overflow: 'hidden',
            border: '2px solid rgba(255,255,255,0.1)',
          }}>
            {!image && children}
          </div>

          {/* Title */}
          <h3 style={{
            fontSize: '20px',
            fontWeight: 'bold',
            margin: '0 0 4px 0',
            background: colors.gradient,
            WebkitBackgroundClip: 'text',
            backgroundClip: 'text',
            color: 'transparent',
            fontFamily: "'Orbitron', monospace",
          }}>
            {title}
          </h3>

          {subtitle && (
            <p style={{
              fontSize: '12px',
              color: '#888',
              margin: '0 0 12px 0',
              fontFamily: 'monospace',
            }}>
              {subtitle}
            </p>
          )}

          {/* Stats */}
          {stats.length > 0 && (
            <div style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(2, 1fr)',
              gap: '8px',
            }}>
              {stats.map((stat, i) => (
                <div key={i} style={{
                  background: 'rgba(0,0,0,0.3)',
                  borderRadius: '6px',
                  padding: '6px 10px',
                  display: 'flex',
                  justifyContent: 'space-between',
                  fontSize: '11px',
                }}>
                  <span style={{ color: '#666' }}>{stat.label}</span>
                  <span style={{ color: colors.border, fontWeight: 'bold' }}>{stat.value}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Edge lighting */}
        <div style={{
          position: 'absolute',
          inset: 0,
          borderRadius: '14px',
          border: '1px solid transparent',
          background: `linear-gradient(145deg, rgba(255,255,255,0.2), transparent 50%) border-box`,
          WebkitMask: 'linear-gradient(#fff 0 0) padding-box, linear-gradient(#fff 0 0)',
          WebkitMaskComposite: 'xor',
          maskComposite: 'exclude',
          pointerEvents: 'none',
        }} />
      </motion.div>
    </motion.div>
  );
};

/**
 * HolographicCardShowcase - Grid of holographic cards
 */
export const HolographicCardShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  const cards = [
    { title: 'X3 VALIDATOR', subtitle: 'Genesis Node #001', rarity: 'mythic' as const, stats: [{ label: 'STAKE', value: '100K' }, { label: 'APY', value: '12.5%' }, { label: 'UPTIME', value: '99.9%' }, { label: 'BLOCKS', value: '1.2M' }] },
    { title: 'QUANTUM CORE', subtitle: 'Processing Unit', rarity: 'legendary' as const, stats: [{ label: 'QUBITS', value: '128' }, { label: 'SPEED', value: '10ns' }, { label: 'ERROR', value: '0.01%' }, { label: 'GATES', value: '∞' }] },
    { title: 'NEURAL AGENT', subtitle: 'AI Unit #42', rarity: 'epic' as const, stats: [{ label: 'IQ', value: '300' }, { label: 'TASKS', value: '10K' }, { label: 'LEARN', value: '99%' }, { label: 'TRUST', value: '97%' }] },
    { title: 'DEFI SHARD', subtitle: 'Liquidity Fragment', rarity: 'rare' as const, stats: [{ label: 'TVL', value: '$2.4M' }, { label: 'VOL', value: '$500K' }, { label: 'FEES', value: '0.3%' }, { label: 'PAIRS', value: '47' }] },
  ];

  return (
    <div className={`holographic-showcase ${className}`}>
      <div className="text-center mb-8">
        <h2 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-yellow-400 via-pink-500 to-cyan-400">
          HOLOGRAPHIC COLLECTION
        </h2>
        <p className="text-gray-400 mt-2">Hover to reveal the holographic effect</p>
      </div>
      
      <div className="flex flex-wrap justify-center gap-8 p-4">
        {cards.map((card, i) => (
          <HolographicCard
            key={i}
            {...card}
          >
            <div style={{
              fontSize: '60px',
              opacity: 0.3,
            }}>
              {i === 0 ? '⚡' : i === 1 ? '🔮' : i === 2 ? '🤖' : '💎'}
            </div>
          </HolographicCard>
        ))}
      </div>
    </div>
  );
};

export default HolographicCard;
