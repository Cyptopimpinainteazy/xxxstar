'use client';

import { motion, useScroll, useTransform } from 'framer-motion';
import React, { useRef } from 'react';

interface SceneItem {
  id: string;
  title: string;
  subtitle?: string;
  image?: string;
  href?: string;
}

interface SceneNavigationProps {
  items: SceneItem[];
  title?: string;
  variant?: 'grid' | 'horizontal' | 'stacked';
  className?: string;
}

export const SceneNavigation: React.FC<SceneNavigationProps> = ({
  items,
  title,
  variant = 'grid',
  className = '',
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ['start end', 'end start'],
  });

  const y1 = useTransform(scrollYProgress, [0, 1], [100, -100]);
  const y2 = useTransform(scrollYProgress, [0, 1], [50, -50]);

  return (
    <section ref={containerRef} className={`scene-navigation relative py-24 overflow-hidden ${className}`}>
      {/* Background Grid Pattern */}
      <div 
        className="absolute inset-0 opacity-10"
        style={{
          backgroundImage: `
            linear-gradient(rgba(0, 255, 255, 0.1) 1px, transparent 1px),
            linear-gradient(90deg, rgba(0, 255, 255, 0.1) 1px, transparent 1px)
          `,
          backgroundSize: '60px 60px',
        }}
      />

      <div className="container mx-auto px-6">
        {/* Section Title with Parallax */}
        {title && (
          <motion.div style={{ y: y2 }} className="mb-16">
            <ParallaxMaskedTitle text={title} />
          </motion.div>
        )}

        {/* Scene Grid */}
        {variant === 'grid' && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
            {items.map((item, index) => (
              <SceneCard key={item.id} item={item} index={index} />
            ))}
          </div>
        )}

        {/* Horizontal Scroll */}
        {variant === 'horizontal' && (
          <div className="flex gap-8 overflow-x-auto pb-8 snap-x snap-mandatory scrollbar-hide">
            {items.map((item, index) => (
              <SceneCard key={item.id} item={item} index={index} className="flex-shrink-0 w-80 snap-center" />
            ))}
          </div>
        )}

        {/* Stacked/Overlapping */}
        {variant === 'stacked' && (
          <div className="relative h-[600px]">
            {items.map((item, index) => (
              <motion.div
                key={item.id}
                className="absolute w-full max-w-md"
                style={{
                  left: `${index * 15}%`,
                  top: `${index * 10}%`,
                  zIndex: items.length - index,
                }}
                initial={{ opacity: 0, y: 50, rotateZ: -5 + index * 2 }}
                whileInView={{ opacity: 1, y: 0, rotateZ: -5 + index * 2 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.15, duration: 0.6 }}
              >
                <SceneCard item={item} index={index} />
              </motion.div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
};

// Individual Scene Card
interface SceneCardProps {
  item: SceneItem;
  index: number;
  className?: string;
}

const SceneCard: React.FC<SceneCardProps> = ({ item, index, className = '' }) => {
  return (
    <motion.a
      href={item.href || '#'}
      className={`scene-card group relative block overflow-hidden rounded-xl bg-gradient-to-br from-gray-900/80 to-black/80 border border-white/10 backdrop-blur-sm ${className}`}
      initial={{ opacity: 0, y: 30 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ delay: index * 0.1, duration: 0.5 }}
      whileHover={{ y: -8 }}
    >
      {/* Image */}
      {item.image && (
        <div className="relative h-48 overflow-hidden">
          <motion.img
            src={item.image}
            alt={item.title}
            className="w-full h-full object-cover"
            whileHover={{ scale: 1.1 }}
            transition={{ duration: 0.6 }}
          />
          <div className="absolute inset-0 bg-gradient-to-t from-black/80 to-transparent" />
        </div>
      )}

      {/* Content */}
      <div className="p-6">
        <div className="flex items-center gap-3 mb-3">
          <span className="w-8 h-8 rounded-full bg-gradient-to-br from-cyan-500 to-purple-500 flex items-center justify-center text-sm font-bold text-white">
            {String(index + 1).padStart(2, '0')}
          </span>
          {item.subtitle && (
            <span className="text-xs uppercase tracking-wider text-cyan-400/70">
              {item.subtitle}
            </span>
          )}
        </div>

        <h3 
          className="text-xl font-bold text-white group-hover:text-cyan-400 transition-colors duration-300"
          style={{ fontFamily: "'Oswald', sans-serif" }}
        >
          {item.title}
        </h3>

        {/* Hover Arrow */}
        <motion.div
          className="mt-4 flex items-center gap-2 text-cyan-400/60"
          initial={{ opacity: 0, x: -10 }}
          whileHover={{ opacity: 1, x: 0 }}
        >
          <span className="text-sm">Navigate</span>
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 8l4 4m0 0l-4 4m4-4H3" />
          </svg>
        </motion.div>
      </div>

      {/* Glow Effect */}
      <motion.div
        className="absolute inset-0 rounded-xl opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"
        style={{
          boxShadow: '0 0 60px rgba(0, 255, 255, 0.2), inset 0 0 60px rgba(0, 255, 255, 0.05)',
        }}
      />
    </motion.a>
  );
};

// Parallax Masked Title Component
interface ParallaxMaskedTitleProps {
  text: string;
  className?: string;
}

export const ParallaxMaskedTitle: React.FC<ParallaxMaskedTitleProps> = ({ text, className = '' }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ['start end', 'end start'],
  });

  const x1 = useTransform(scrollYProgress, [0, 1], [-50, 50]);
  const x2 = useTransform(scrollYProgress, [0, 1], [50, -50]);

  return (
    <div ref={containerRef} className={`relative overflow-hidden ${className}`}>
      {/* Background Text - Large, Faded */}
      <motion.div
        style={{ x: x1 }}
        className="text-6xl md:text-8xl lg:text-9xl font-black text-white/5 uppercase tracking-tighter whitespace-nowrap"
      >
        {text}
      </motion.div>

      {/* Foreground Text - Smaller, Visible */}
      <motion.h2
        style={{ x: x2 }}
        className="absolute top-1/2 left-0 -translate-y-1/2 text-4xl md:text-5xl lg:text-6xl font-bold text-white uppercase tracking-tight"
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true }}
      >
        {text}
      </motion.h2>

      {/* Accent Line */}
      <motion.div
        className="absolute bottom-0 left-0 h-1 bg-gradient-to-r from-cyan-500 via-purple-500 to-cyan-500"
        initial={{ width: 0 }}
        whileInView={{ width: '30%' }}
        viewport={{ once: true }}
        transition={{ duration: 0.8, delay: 0.3 }}
      />
    </div>
  );
};

// Scene Navigation Dots
interface SceneDotsProps {
  total: number;
  current: number;
  onChange?: (index: number) => void;
  className?: string;
}

export const SceneDots: React.FC<SceneDotsProps> = ({
  total,
  current,
  onChange,
  className = '',
}) => {
  return (
    <div className={`flex items-center gap-3 ${className}`}>
      {Array.from({ length: total }).map((_, index) => (
        <button
          key={index}
          onClick={() => onChange?.(index)}
          className={`relative w-3 h-3 rounded-full transition-all duration-300 ${
            index === current
              ? 'bg-cyan-400 scale-125'
              : 'bg-white/30 hover:bg-white/50'
          }`}
        >
          {index === current && (
            <motion.span
              className="absolute inset-0 rounded-full bg-cyan-400"
              layoutId="sceneDot"
              transition={{ type: 'spring', stiffness: 300, damping: 30 }}
            />
          )}
        </button>
      ))}
    </div>
  );
};
