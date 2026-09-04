'use client';

import { motion } from 'framer-motion';
import React, { useState, useEffect } from 'react';

interface GlitchTextProps {
  text: string;
  as?: 'h1' | 'h2' | 'h3' | 'h4' | 'p' | 'span';
  className?: string;
  glitchIntensity?: 'low' | 'medium' | 'high';
  continuous?: boolean;
  triggerOnHover?: boolean;
}

export const GlitchText: React.FC<GlitchTextProps> = ({
  text,
  as: Component = 'span',
  className = '',
  glitchIntensity = 'medium',
  continuous = false,
  triggerOnHover = true,
}) => {
  const [isGlitching, setIsGlitching] = useState(continuous);

  const intensityConfig = {
    low: { frequency: 5000, duration: 150 },
    medium: { frequency: 3000, duration: 200 },
    high: { frequency: 1500, duration: 300 },
  };

  useEffect(() => {
    if (!continuous) return;
    const interval = setInterval(() => {
      setIsGlitching(true);
      setTimeout(() => setIsGlitching(false), intensityConfig[glitchIntensity].duration);
    }, intensityConfig[glitchIntensity].frequency);
    return () => clearInterval(interval);
  }, [continuous, glitchIntensity]);

  return (
    <Component
      className={`glitch-text relative inline-block ${className}`}
      onMouseEnter={() => triggerOnHover && setIsGlitching(true)}
      onMouseLeave={() => triggerOnHover && !continuous && setIsGlitching(false)}
    >
      {/* Base Text */}
      <span className="relative z-10">{text}</span>

      {/* Glitch Layers */}
      {isGlitching && (
        <>
          <motion.span
            className="absolute inset-0 text-red-500/80"
            style={{ clipPath: 'inset(20% 0 30% 0)' }}
            animate={{
              x: [-2, 2, -2, 1, 0],
              opacity: [0.8, 0.6, 0.8, 0.7, 0],
            }}
            transition={{ duration: 0.2, repeat: Infinity }}
          >
            {text}
          </motion.span>
          <motion.span
            className="absolute inset-0 text-cyan-500/80"
            style={{ clipPath: 'inset(50% 0 20% 0)' }}
            animate={{
              x: [2, -2, 1, -1, 0],
              opacity: [0.7, 0.8, 0.6, 0.8, 0],
            }}
            transition={{ duration: 0.2, repeat: Infinity, delay: 0.05 }}
          >
            {text}
          </motion.span>
        </>
      )}
    </Component>
  );
};

// Typewriter Effect
interface TypewriterTextProps {
  text: string;
  speed?: number;
  delay?: number;
  cursor?: boolean;
  className?: string;
  onComplete?: () => void;
}

export const TypewriterText: React.FC<TypewriterTextProps> = ({
  text,
  speed = 50,
  delay = 0,
  cursor = true,
  className = '',
  onComplete,
}) => {
  const [displayText, setDisplayText] = useState('');
  const [isComplete, setIsComplete] = useState(false);

  useEffect(() => {
    let timeout: NodeJS.Timeout;
    let index = 0;

    const startTyping = () => {
      timeout = setInterval(() => {
        if (index < text.length) {
          setDisplayText(text.slice(0, index + 1));
          index++;
        } else {
          clearInterval(timeout);
          setIsComplete(true);
          onComplete?.();
        }
      }, speed);
    };

    const delayTimeout = setTimeout(startTyping, delay);

    return () => {
      clearTimeout(delayTimeout);
      clearInterval(timeout);
    };
  }, [text, speed, delay, onComplete]);

  return (
    <span className={`typewriter-text ${className}`}>
      {displayText}
      {cursor && !isComplete && (
        <motion.span
          className="inline-block w-[2px] h-[1em] bg-current ml-1"
          animate={{ opacity: [1, 0] }}
          transition={{ duration: 0.5, repeat: Infinity }}
        />
      )}
    </span>
  );
};

// Scramble Text Effect
interface ScrambleTextProps {
  text: string;
  className?: string;
  scrambleOnHover?: boolean;
  duration?: number;
}

export const ScrambleText: React.FC<ScrambleTextProps> = ({
  text,
  className = '',
  scrambleOnHover = true,
  duration = 1000,
}) => {
  const [displayText, setDisplayText] = useState(text);
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*';

  const scramble = () => {
    let iteration = 0;
    const totalIterations = duration / 30;
    
    const interval = setInterval(() => {
      setDisplayText(prev =>
        text
          .split('')
          .map((char, index) => {
            if (index < iteration) {
              return text[index];
            }
            return chars[Math.floor(Math.random() * chars.length)];
          })
          .join('')
      );

      if (iteration >= text.length) {
        clearInterval(interval);
        setDisplayText(text);
      }

      iteration += text.length / totalIterations;
    }, 30);
  };

  return (
    <span
      className={`scramble-text font-mono ${className}`}
      onMouseEnter={() => scrambleOnHover && scramble()}
    >
      {displayText}
    </span>
  );
};

// Gradient Animated Text
interface GradientTextProps {
  text: string;
  as?: 'h1' | 'h2' | 'h3' | 'h4' | 'p' | 'span';
  gradient?: string;
  animated?: boolean;
  className?: string;
}

export const GradientText: React.FC<GradientTextProps> = ({
  text,
  as: Component = 'span',
  gradient = 'from-cyan-400 via-purple-500 to-pink-500',
  animated = true,
  className = '',
}) => {
  return (
    <Component className={`gradient-text relative ${className}`}>
      <motion.span
        className={`bg-gradient-to-r ${gradient} bg-clip-text text-transparent`}
        style={{
          backgroundSize: animated ? '200% 200%' : '100% 100%',
        }}
        animate={animated ? {
          backgroundPosition: ['0% 50%', '100% 50%', '0% 50%'],
        } : {}}
        transition={{
          duration: 5,
          repeat: Infinity,
          ease: 'linear',
        }}
      >
        {text}
      </motion.span>
    </Component>
  );
};

// Split Letter Animation
interface SplitTextProps {
  text: string;
  className?: string;
  staggerDelay?: number;
}

export const SplitText: React.FC<SplitTextProps> = ({
  text,
  className = '',
  staggerDelay = 0.03,
}) => {
  return (
    <span className={`split-text inline-block ${className}`}>
      {text.split('').map((char, index) => (
        <motion.span
          key={index}
          className="inline-block"
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ delay: index * staggerDelay, duration: 0.3 }}
        >
          {char === ' ' ? '\u00A0' : char}
        </motion.span>
      ))}
    </span>
  );
};
