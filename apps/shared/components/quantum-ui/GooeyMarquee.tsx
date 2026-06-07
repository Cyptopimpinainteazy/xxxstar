'use client';

import React from 'react';
import { motion } from 'framer-motion';

interface GooeyMarqueeProps {
  text: string;
  speed?: number;
  fontSize?: string;
  className?: string;
  direction?: 'left' | 'right';
  color?: string;
}

/**
 * GooeyMarquee - Scrolling text with gooey blur/contrast SVG filter effect
 * 
 * The magic: SVG filter with blur + color matrix creates the liquid blob effect
 * Text color MUST match the edge color for the gooey merge to work!
 */
export const GooeyMarquee: React.FC<GooeyMarqueeProps> = ({
  text,
  speed = 20,
  fontSize = '4rem',
  className = '',
  direction = 'left',
  color = '#ffffff',
}) => {
  const animationDirection = direction === 'left' ? ['100%', '-100%'] : ['-100%', '100%'];
  const filterId = `gooey-${Math.random().toString(36).substr(2, 9)}`;

  return (
    <div
      className={`gooey-marquee-wrapper ${className}`}
      style={{
        position: 'relative',
        width: '100%',
        overflow: 'hidden',
        background: '#000',
      }}
    >
      {/* SVG Filter Definition - This is the secret sauce! */}
      <svg style={{ position: 'absolute', width: 0, height: 0 }}>
        <defs>
          <filter id={filterId}>
            {/* Blur the content */}
            <feGaussianBlur in="SourceGraphic" stdDeviation="10" result="blur" />
            {/* High contrast color matrix - makes blurred edges merge */}
            <feColorMatrix
              in="blur"
              mode="matrix"
              values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 19 -9"
              result="goo"
            />
            {/* Composite original on top for crisp text */}
            <feComposite in="SourceGraphic" in2="goo" operator="atop" />
          </filter>
        </defs>
      </svg>

      {/* Container with the gooey filter applied */}
      <div
        style={{
          position: 'relative',
          filter: `url(#${filterId})`,
          display: 'flex',
          alignItems: 'center',
          height: fontSize,
          padding: '1rem 0',
        }}
      >
        {/* Left edge blob - same color as text! */}
        <div
          style={{
            position: 'absolute',
            left: 0,
            top: '50%',
            transform: 'translateY(-50%)',
            width: '4rem',
            height: '80%',
            background: color,
            borderRadius: '0 50% 50% 0',
            zIndex: 2,
          }}
        />

        {/* Right edge blob - same color as text! */}
        <div
          style={{
            position: 'absolute',
            right: 0,
            top: '50%',
            transform: 'translateY(-50%)',
            width: '4rem',
            height: '80%',
            background: color,
            borderRadius: '50% 0 0 50%',
            zIndex: 2,
          }}
        />

        {/* Scrolling text - SAME COLOR as edge blobs! */}
        <motion.div
          style={{
            display: 'flex',
            whiteSpace: 'nowrap',
            fontSize,
            fontFamily: "'Orbitron', 'Raleway', sans-serif",
            fontWeight: 900,
            color: color,
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
          }}
          animate={{ x: animationDirection }}
          transition={{
            duration: speed,
            repeat: Infinity,
            ease: 'linear',
          }}
        >
          <span style={{ paddingRight: '2rem' }}>{text}</span>
          <span style={{ paddingRight: '2rem' }}>{text}</span>
          <span style={{ paddingRight: '2rem' }}>{text}</span>
        </motion.div>
      </div>
    </div>
  );
};

/**
 * ColorfulGooeyMarquee - Gooey effect with color overlay
 */
export const ColorfulGooeyMarquee: React.FC<{
  text: string;
  speed?: number;
  fontSize?: string;
  className?: string;
  direction?: 'left' | 'right';
  gradientStart?: string;
  gradientEnd?: string;
}> = ({
  text,
  speed = 20,
  fontSize = '4rem',
  className = '',
  direction = 'left',
  gradientStart = '#00ffea',
  gradientEnd = '#ff00ff',
}) => {
  return (
    <div className={className} style={{ position: 'relative' }}>
      {/* White gooey layer underneath */}
      <GooeyMarquee
        text={text}
        speed={speed}
        fontSize={fontSize}
        direction={direction}
        color="#ffffff"
      />
      
      {/* Gradient overlay using mix-blend-mode */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          background: `linear-gradient(90deg, ${gradientStart}, ${gradientEnd})`,
          mixBlendMode: 'multiply',
          pointerEvents: 'none',
        }}
      />
    </div>
  );
};

/**
 * GooeyMarqueeStack - Multiple marquees stacked
 */
export const GooeyMarqueeStack: React.FC<{
  lines: Array<{ text: string; speed?: number; color?: string }>;
  className?: string;
}> = ({ lines, className = '' }) => {
  return (
    <div className={`gooey-marquee-stack ${className}`} style={{ display: 'flex', flexDirection: 'column' }}>
      {lines.map((line, i) => (
        <GooeyMarquee
          key={i}
          text={line.text}
          speed={line.speed || 15 + i * 5}
          direction={i % 2 === 0 ? 'left' : 'right'}
          color={line.color || '#ffffff'}
          fontSize={`${4 - i * 0.5}rem`}
        />
      ))}
    </div>
  );
};

/**
 * GooeyMarqueeShowcase - Display the effect
 */
export const GooeyMarqueeShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div
      className={`gooey-marquee-showcase ${className}`}
      style={{
        background: '#000',
        padding: '2rem 0',
        borderRadius: '1rem',
        overflow: 'hidden',
      }}
    >
      {/* Pure white gooey - the classic effect */}
      <GooeyMarquee
        text="X3 CHAIN • QUANTUM BLOCKCHAIN • YEAR 2060 • "
        speed={25}
        fontSize="5rem"
        color="#ffffff"
      />

      <div style={{ height: '1rem' }} />

      {/* Cyan gooey */}
      <GooeyMarquee
        text="DUAL-VM EXECUTION • EVM + SVM • ATOMIC TRANSACTIONS • "
        speed={20}
        fontSize="3rem"
        direction="right"
        color="#00ffea"
      />

      <div style={{ height: '1rem' }} />

      {/* Pink/magenta gooey */}
      <GooeyMarquee
        text="GPU SWARM • AI POWERED • 103+ CHAINS • CROSS-CHAIN DEFI • "
        speed={18}
        fontSize="2.5rem"
        color="#ff00ff"
      />
      
      <div style={{ height: '1rem' }} />

      {/* Yellow gooey */}
      <GooeyMarquee
        text="⚡ BLOCKCHAIN OF THE FUTURE ⚡ NEURAL CONSENSUS ⚡ "
        speed={22}
        fontSize="3.5rem"
        direction="right"
        color="#ffd93d"
      />
    </div>
  );
};

export default GooeyMarquee;
