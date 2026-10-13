'use client';

import React, { useRef, ReactNode } from 'react';
import { motion, useScroll, useTransform, useSpring, MotionValue } from 'framer-motion';

interface KineticScrollProps {
  children: ReactNode;
  effect?: 'zoom' | 'spin' | 'slide' | 'fade' | 'parallax' | 'reveal' | 'explode' | 'flip' | 'wave';
  direction?: 'up' | 'down' | 'left' | 'right';
  delay?: number;
  duration?: number;
  className?: string;
  intensity?: number;
}

export function KineticScroll({
  children,
  effect = 'fade',
  direction = 'up',
  delay = 0,
  duration = 0.6,
  className = '',
  intensity = 1
}: KineticScrollProps) {
  const ref = useRef<HTMLDivElement>(null);
  
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ["start end", "end start"]
  });
  
  // Smooth spring physics
  const smoothProgress = useSpring(scrollYProgress, {
    stiffness: 100,
    damping: 30,
    restDelta: 0.001
  });
  
  const getTransforms = (): Record<string, MotionValue<number | string>> => {
    switch (effect) {
      case 'zoom':
        return {
          scale: useTransform(smoothProgress, [0, 0.5, 1], [0.5 / intensity, 1, 0.8]),
          opacity: useTransform(smoothProgress, [0, 0.3, 0.7, 1], [0, 1, 1, 0])
        };
        
      case 'spin':
        return {
          rotate: useTransform(smoothProgress, [0, 1], [360 * intensity, 0]),
          scale: useTransform(smoothProgress, [0, 0.5, 1], [0.3, 1, 0.8]),
          opacity: useTransform(smoothProgress, [0, 0.3, 0.7, 1], [0, 1, 1, 0])
        };
        
      case 'slide':
        const xOffset = direction === 'left' ? -200 : direction === 'right' ? 200 : 0;
        const yOffset = direction === 'up' ? 200 : direction === 'down' ? -200 : 0;
        return {
          x: useTransform(smoothProgress, [0, 0.5], [xOffset * intensity, 0]),
          y: useTransform(smoothProgress, [0, 0.5], [yOffset * intensity, 0]),
          opacity: useTransform(smoothProgress, [0, 0.3, 0.7, 1], [0, 1, 1, 0.3])
        };
        
      case 'parallax':
        return {
          y: useTransform(smoothProgress, [0, 1], [100 * intensity, -100 * intensity])
        };
        
      case 'reveal':
        return {
          clipPath: useTransform(
            smoothProgress,
            [0, 0.5],
            ['inset(100% 0% 0% 0%)', 'inset(0% 0% 0% 0%)']
          ),
          opacity: useTransform(smoothProgress, [0, 0.3], [0, 1])
        };
        
      case 'explode':
        return {
          scale: useTransform(smoothProgress, [0, 0.4, 0.5, 0.6], [0, 1.5, 1, 1]),
          opacity: useTransform(smoothProgress, [0, 0.3, 0.7, 1], [0, 1, 1, 0]),
          rotate: useTransform(smoothProgress, [0, 0.5], [-10 * intensity, 0])
        };
        
      case 'flip':
        return {
          rotateX: useTransform(smoothProgress, [0, 0.5], [90 * intensity, 0]),
          opacity: useTransform(smoothProgress, [0, 0.3, 0.7, 1], [0, 1, 1, 0]),
          perspective: useTransform(smoothProgress, [0, 1], [1000, 1000])
        };
        
      case 'wave':
        return {
          y: useTransform(
            smoothProgress,
            [0, 0.25, 0.5, 0.75, 1],
            [50 * intensity, -20 * intensity, 50 * intensity, -20 * intensity, 0]
          ),
          opacity: useTransform(smoothProgress, [0, 0.2, 0.8, 1], [0, 1, 1, 0.5])
        };
        
      case 'fade':
      default:
        return {
          opacity: useTransform(smoothProgress, [0, 0.3, 0.7, 1], [0, 1, 1, 0]),
          y: useTransform(smoothProgress, [0, 0.3], [50, 0])
        };
    }
  };
  
  const transforms = getTransforms();
  
  return (
    <motion.div
      ref={ref}
      style={transforms}
      className={className}
      transition={{ duration, delay }}
    >
      {children}
    </motion.div>
  );
}

// Staggered children animation
interface StaggeredContainerProps {
  children: ReactNode;
  staggerDelay?: number;
  className?: string;
}

export function StaggeredContainer({ 
  children, 
  staggerDelay = 0.1,
  className = ''
}: StaggeredContainerProps) {
  return (
    <motion.div
      className={className}
      initial="hidden"
      whileInView="visible"
      viewport={{ once: true, margin: "-100px" }}
      variants={{
        hidden: { opacity: 0 },
        visible: {
          opacity: 1,
          transition: {
            staggerChildren: staggerDelay
          }
        }
      }}
    >
      {children}
    </motion.div>
  );
}

interface StaggeredItemProps {
  children: ReactNode;
  effect?: 'fade-up' | 'fade-down' | 'fade-left' | 'fade-right' | 'scale' | 'rotate';
  className?: string;
}

export function StaggeredItem({ 
  children, 
  effect = 'fade-up',
  className = ''
}: StaggeredItemProps) {
  const getVariants = () => {
    switch (effect) {
      case 'fade-up':
        return {
          hidden: { opacity: 0, y: 50 },
          visible: { opacity: 1, y: 0, transition: { duration: 0.5 } }
        };
      case 'fade-down':
        return {
          hidden: { opacity: 0, y: -50 },
          visible: { opacity: 1, y: 0, transition: { duration: 0.5 } }
        };
      case 'fade-left':
        return {
          hidden: { opacity: 0, x: -50 },
          visible: { opacity: 1, x: 0, transition: { duration: 0.5 } }
        };
      case 'fade-right':
        return {
          hidden: { opacity: 0, x: 50 },
          visible: { opacity: 1, x: 0, transition: { duration: 0.5 } }
        };
      case 'scale':
        return {
          hidden: { opacity: 0, scale: 0.5 },
          visible: { opacity: 1, scale: 1, transition: { duration: 0.5 } }
        };
      case 'rotate':
        return {
          hidden: { opacity: 0, rotate: -10, scale: 0.9 },
          visible: { opacity: 1, rotate: 0, scale: 1, transition: { duration: 0.5 } }
        };
      default:
        return {
          hidden: { opacity: 0 },
          visible: { opacity: 1 }
        };
    }
  };
  
  return (
    <motion.div
      variants={getVariants()}
      className={className}
    >
      {children}
    </motion.div>
  );
}

// Particle explosion effect
interface ParticleExplosionProps {
  trigger?: boolean;
  color?: string;
  particleCount?: number;
  className?: string;
}

export function ParticleExplosion({
  trigger = false,
  color = '#00f3ff',
  particleCount = 20,
  className = ''
}: ParticleExplosionProps) {
  if (!trigger) return null;
  
  return (
    <div className={`absolute inset-0 pointer-events-none ${className}`}>
      {Array.from({ length: particleCount }).map((_, i) => (
        <motion.div
          key={i}
          className="absolute w-2 h-2 rounded-full"
          style={{ 
            backgroundColor: color,
            left: '50%',
            top: '50%',
            boxShadow: `0 0 10px ${color}`
          }}
          initial={{ scale: 0, x: 0, y: 0 }}
          animate={{
            scale: [0, 1, 0],
            x: (Math.random() - 0.5) * 300,
            y: (Math.random() - 0.5) * 300,
            opacity: [1, 0]
          }}
          transition={{
            duration: 0.8,
            delay: i * 0.02,
            ease: 'easeOut'
          }}
        />
      ))}
    </div>
  );
}

// Text reveal animation
interface TextRevealProps {
  text: string;
  className?: string;
  delay?: number;
}

export function TextReveal({ text, className = '', delay = 0 }: TextRevealProps) {
  const words = text.split(' ');
  
  return (
    <motion.span
      initial="hidden"
      whileInView="visible"
      viewport={{ once: true }}
      className={className}
    >
      {words.map((word, i) => (
        <motion.span
          key={i}
          className="inline-block mr-2"
          variants={{
            hidden: { opacity: 0, y: 20 },
            visible: {
              opacity: 1,
              y: 0,
              transition: {
                delay: delay + i * 0.05,
                duration: 0.3
              }
            }
          }}
        >
          {word}
        </motion.span>
      ))}
    </motion.span>
  );
}

// Counter animation
interface AnimatedCounterProps {
  from?: number;
  to: number;
  duration?: number;
  suffix?: string;
  prefix?: string;
  className?: string;
}

export function AnimatedCounter({
  from = 0,
  to,
  duration = 2,
  suffix = '',
  prefix = '',
  className = ''
}: AnimatedCounterProps) {
  const ref = useRef<HTMLSpanElement>(null);
  
  return (
    <motion.span
      ref={ref}
      className={className}
      initial={{ opacity: 0 }}
      whileInView={{ opacity: 1 }}
      viewport={{ once: true }}
    >
      <motion.span
        initial={{ opacity: 0 }}
        whileInView={{
          opacity: 1,
          transition: { duration: 0.3 }
        }}
        viewport={{ once: true }}
      >
        {prefix}
        <motion.span
          initial={{ opacity: 1 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          onViewportEnter={() => {
            if (!ref.current) return;
            
            const startTime = performance.now();
            const animate = (currentTime: number) => {
              const elapsed = currentTime - startTime;
              const progress = Math.min(elapsed / (duration * 1000), 1);
              
              // Easing function
              const eased = 1 - Math.pow(1 - progress, 3);
              const current = Math.floor(from + (to - from) * eased);
              
              if (ref.current) {
                ref.current.textContent = `${prefix}${current.toLocaleString()}${suffix}`;
              }
              
              if (progress < 1) {
                requestAnimationFrame(animate);
              }
            };
            
            requestAnimationFrame(animate);
          }}
        />
        {suffix}
      </motion.span>
    </motion.span>
  );
}

// Magnetic hover effect
interface MagneticProps {
  children: ReactNode;
  strength?: number;
  className?: string;
}

export function Magnetic({ 
  children, 
  strength = 0.3,
  className = ''
}: MagneticProps) {
  const ref = useRef<HTMLDivElement>(null);
  
  const handleMouseMove = (e: React.MouseEvent) => {
    if (!ref.current) return;
    
    const rect = ref.current.getBoundingClientRect();
    const x = e.clientX - rect.left - rect.width / 2;
    const y = e.clientY - rect.top - rect.height / 2;
    
    ref.current.style.transform = `translate(${x * strength}px, ${y * strength}px)`;
  };
  
  const handleMouseLeave = () => {
    if (!ref.current) return;
    ref.current.style.transform = 'translate(0, 0)';
  };
  
  return (
    <motion.div
      ref={ref}
      className={`transition-transform duration-200 ${className}`}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      {children}
    </motion.div>
  );
}

// Export all components
export default {
  KineticScroll,
  StaggeredContainer,
  StaggeredItem,
  ParticleExplosion,
  TextReveal,
  AnimatedCounter,
  Magnetic
};
