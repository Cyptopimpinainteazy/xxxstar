'use client';

import { motion } from 'framer-motion';
import React, { useState } from 'react';

interface PosterArtCardProps {
  image: string;
  title: string;
  subtitle?: string;
  description?: string;
  href?: string;
  badge?: string;
  className?: string;
  aspectRatio?: 'portrait' | 'landscape' | 'square';
}

export const PosterArtCard: React.FC<PosterArtCardProps> = ({
  image,
  title,
  subtitle,
  description,
  href = '#',
  badge,
  className = '',
  aspectRatio = 'portrait',
}) => {
  const [isHovered, setIsHovered] = useState(false);

  const aspectClasses = {
    portrait: 'aspect-[3/4]',
    landscape: 'aspect-[4/3]',
    square: 'aspect-square',
  };

  return (
    <motion.a
      href={href}
      className={`poster-art-card group relative block overflow-hidden rounded-xl ${aspectClasses[aspectRatio]} ${className}`}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ duration: 0.6 }}
    >
      {/* Background Image with Zoom */}
      <motion.div
        className="absolute inset-0 bg-cover bg-center"
        style={{ backgroundImage: `url(${image})` }}
        animate={{ scale: isHovered ? 1.1 : 1 }}
        transition={{ duration: 0.6, ease: [0.25, 0.46, 0.45, 0.94] }}
      />

      {/* Gradient Overlay */}
      <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/40 to-transparent" />

      {/* Badge */}
      {badge && (
        <motion.div
          className="absolute top-4 left-4 px-3 py-1 rounded-full bg-gradient-to-r from-cyan-500/80 to-purple-500/80 backdrop-blur-sm text-xs font-bold uppercase tracking-wider text-white"
          animate={{ y: isHovered ? 0 : -8, opacity: isHovered ? 1 : 0.8 }}
          transition={{ duration: 0.3 }}
        >
          {badge}
        </motion.div>
      )}

      {/* Content Container */}
      <div className="absolute bottom-0 left-0 right-0 p-6">
        {/* Subtitle */}
        {subtitle && (
          <motion.p
            className="text-xs font-medium uppercase tracking-[0.3em] text-cyan-400 mb-2"
            animate={{ opacity: isHovered ? 1 : 0.6, y: isHovered ? 0 : 10 }}
            transition={{ duration: 0.4, delay: 0.05 }}
          >
            {subtitle}
          </motion.p>
        )}

        {/* Title with Slide Effect */}
        <motion.h3
          className="text-2xl md:text-3xl font-bold text-white mb-2"
          style={{ fontFamily: "'Oswald', sans-serif" }}
          animate={{ y: isHovered ? -8 : 0 }}
          transition={{ duration: 0.4, ease: 'easeOut' }}
        >
          {title}
        </motion.h3>

        {/* Description - Reveals on Hover */}
        {description && (
          <motion.p
            className="text-sm text-gray-300 line-clamp-2"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: isHovered ? 1 : 0, y: isHovered ? 0 : 20 }}
            transition={{ duration: 0.4, delay: 0.1 }}
          >
            {description}
          </motion.p>
        )}

        {/* CTA Arrow */}
        <motion.div
          className="mt-4 flex items-center gap-2 text-cyan-400 font-medium"
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: isHovered ? 1 : 0, x: isHovered ? 0 : -20 }}
          transition={{ duration: 0.3, delay: 0.15 }}
        >
          <span className="text-sm uppercase tracking-wider">Explore</span>
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 8l4 4m0 0l-4 4m4-4H3" />
          </svg>
        </motion.div>
      </div>

      {/* Scan Line Effect */}
      <motion.div
        className="absolute inset-0 pointer-events-none"
        style={{
          background: 'linear-gradient(transparent 50%, rgba(0, 255, 255, 0.03) 50%)',
          backgroundSize: '100% 4px',
        }}
        animate={{ opacity: isHovered ? 0.5 : 0 }}
        transition={{ duration: 0.3 }}
      />
    </motion.a>
  );
};

// Grid layout component for multiple poster cards
interface PosterArtGridProps {
  children: React.ReactNode;
  columns?: 2 | 3 | 4;
  className?: string;
}

export const PosterArtGrid: React.FC<PosterArtGridProps> = ({
  children,
  columns = 3,
  className = '',
}) => {
  const gridCols = {
    2: 'md:grid-cols-2',
    3: 'md:grid-cols-3',
    4: 'md:grid-cols-4',
  };

  return (
    <div className={`grid grid-cols-1 ${gridCols[columns]} gap-6 ${className}`}>
      {children}
    </div>
  );
};
