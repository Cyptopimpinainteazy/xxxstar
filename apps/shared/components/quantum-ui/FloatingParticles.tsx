'use client';

import { motion } from 'framer-motion';
import React, { useEffect, useRef } from 'react';

interface Particle {
  id: number;
  x: number;
  y: number;
  size: number;
  color: string;
  duration: number;
  delay: number;
}

interface FloatingParticlesProps {
  count?: number;
  colors?: string[];
  minSize?: number;
  maxSize?: number;
  className?: string;
  speed?: 'slow' | 'normal' | 'fast';
}

export const FloatingParticles: React.FC<FloatingParticlesProps> = ({
  count = 50,
  colors = ['#00ffff', '#ff00ff', '#ffff00', '#00ff00', '#ff6600'],
  minSize = 2,
  maxSize = 6,
  className = '',
  speed = 'normal',
}) => {
  const particles: Particle[] = Array.from({ length: count }, (_, i) => ({
    id: i,
    x: Math.random() * 100,
    y: Math.random() * 100,
    size: Math.random() * (maxSize - minSize) + minSize,
    color: colors[Math.floor(Math.random() * colors.length)],
    duration: speed === 'slow' ? 20 + Math.random() * 20 : speed === 'fast' ? 5 + Math.random() * 10 : 10 + Math.random() * 15,
    delay: Math.random() * 5,
  }));

  return (
    <div className={`floating-particles absolute inset-0 overflow-hidden pointer-events-none ${className}`}>
      {particles.map((particle) => (
        <motion.div
          key={particle.id}
          className="absolute rounded-full"
          style={{
            left: `${particle.x}%`,
            top: `${particle.y}%`,
            width: particle.size,
            height: particle.size,
            backgroundColor: particle.color,
            boxShadow: `0 0 ${particle.size * 2}px ${particle.color}`,
          }}
          animate={{
            y: [0, -200, 0],
            x: [0, Math.random() * 100 - 50, 0],
            opacity: [0.3, 0.8, 0.3],
            scale: [1, 1.5, 1],
          }}
          transition={{
            duration: particle.duration,
            delay: particle.delay,
            repeat: Infinity,
            ease: 'easeInOut',
          }}
        />
      ))}
    </div>
  );
};

// Connecting Lines Particles (Network Effect)
interface NetworkParticlesProps {
  nodeCount?: number;
  connectionDistance?: number;
  nodeColor?: string;
  lineColor?: string;
  className?: string;
}

export const NetworkParticles: React.FC<NetworkParticlesProps> = ({
  nodeCount = 30,
  connectionDistance = 150,
  nodeColor = '#00ffff',
  lineColor = 'rgba(0, 255, 255, 0.2)',
  className = '',
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    canvas.width = canvas.offsetWidth;
    canvas.height = canvas.offsetHeight;

    interface Node {
      x: number;
      y: number;
      vx: number;
      vy: number;
    }

    const nodes: Node[] = Array.from({ length: nodeCount }, () => ({
      x: Math.random() * canvas.width,
      y: Math.random() * canvas.height,
      vx: (Math.random() - 0.5) * 0.5,
      vy: (Math.random() - 0.5) * 0.5,
    }));

    const animate = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      // Update positions
      nodes.forEach((node) => {
        node.x += node.vx;
        node.y += node.vy;

        if (node.x < 0 || node.x > canvas.width) node.vx *= -1;
        if (node.y < 0 || node.y > canvas.height) node.vy *= -1;
      });

      // Draw connections
      ctx.strokeStyle = lineColor;
      ctx.lineWidth = 1;
      nodes.forEach((node, i) => {
        nodes.slice(i + 1).forEach((other) => {
          const dist = Math.hypot(node.x - other.x, node.y - other.y);
          if (dist < connectionDistance) {
            ctx.globalAlpha = 1 - dist / connectionDistance;
            ctx.beginPath();
            ctx.moveTo(node.x, node.y);
            ctx.lineTo(other.x, other.y);
            ctx.stroke();
          }
        });
      });

      // Draw nodes
      ctx.globalAlpha = 1;
      ctx.fillStyle = nodeColor;
      nodes.forEach((node) => {
        ctx.beginPath();
        ctx.arc(node.x, node.y, 3, 0, Math.PI * 2);
        ctx.fill();
      });

      requestAnimationFrame(animate);
    };

    const animationId = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animationId);
  }, [nodeCount, connectionDistance, nodeColor, lineColor]);

  return (
    <canvas
      ref={canvasRef}
      className={`network-particles absolute inset-0 w-full h-full pointer-events-none ${className}`}
    />
  );
};

// Rising Bubbles
interface RisingBubblesProps {
  count?: number;
  color?: string;
  className?: string;
}

export const RisingBubbles: React.FC<RisingBubblesProps> = ({
  count = 20,
  color = 'cyan',
  className = '',
}) => {
  const colors: Record<string, { bg: string; glow: string }> = {
    cyan: { bg: 'bg-cyan-500/30', glow: 'rgba(0, 255, 255, 0.3)' },
    purple: { bg: 'bg-purple-500/30', glow: 'rgba(147, 51, 234, 0.3)' },
    gold: { bg: 'bg-yellow-500/30', glow: 'rgba(234, 179, 8, 0.3)' },
  };

  const colorConfig = colors[color] || colors.cyan;

  return (
    <div className={`rising-bubbles absolute inset-0 overflow-hidden pointer-events-none ${className}`}>
      {Array.from({ length: count }).map((_, i) => (
        <motion.div
          key={i}
          className={`absolute rounded-full ${colorConfig.bg} backdrop-blur-sm`}
          style={{
            left: `${Math.random() * 100}%`,
            bottom: -20,
            width: Math.random() * 20 + 10,
            height: Math.random() * 20 + 10,
            boxShadow: `0 0 10px ${colorConfig.glow}`,
          }}
          animate={{
            y: [0, -800],
            x: [0, Math.sin(i) * 50],
            opacity: [0, 1, 1, 0],
          }}
          transition={{
            duration: 8 + Math.random() * 8,
            delay: Math.random() * 10,
            repeat: Infinity,
            ease: 'linear',
          }}
        />
      ))}
    </div>
  );
};

// Sparkle Effect
interface SparkleProps {
  className?: string;
  density?: 'low' | 'medium' | 'high';
  color?: string;
}

export const Sparkles: React.FC<SparkleProps> = ({
  className = '',
  density = 'medium',
  color = '#fff',
}) => {
  const counts = { low: 10, medium: 20, high: 40 };
  const count = counts[density];

  return (
    <div className={`sparkles absolute inset-0 overflow-hidden pointer-events-none ${className}`}>
      {Array.from({ length: count }).map((_, i) => (
        <motion.div
          key={i}
          className="absolute"
          style={{
            left: `${Math.random() * 100}%`,
            top: `${Math.random() * 100}%`,
          }}
          animate={{
            scale: [0, 1, 0],
            opacity: [0, 1, 0],
            rotate: [0, 180],
          }}
          transition={{
            duration: 2 + Math.random() * 2,
            delay: Math.random() * 3,
            repeat: Infinity,
          }}
        >
          <svg
            className="w-3 h-3"
            viewBox="0 0 24 24"
            fill={color}
          >
            <path d="M12 0L14.59 9.41L24 12L14.59 14.59L12 24L9.41 14.59L0 12L9.41 9.41L12 0Z" />
          </svg>
        </motion.div>
      ))}
    </div>
  );
};

// Data Stream (Matrix-like falling characters)
interface DataStreamProps {
  className?: string;
  columns?: number;
  color?: string;
  speed?: 'slow' | 'normal' | 'fast';
}

export const DataStream: React.FC<DataStreamProps> = ({
  className = '',
  columns = 20,
  color = '#00ff00',
  speed = 'normal',
}) => {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@#$%^&*';
  const speeds = { slow: 15, normal: 8, fast: 4 };

  return (
    <div className={`data-stream absolute inset-0 overflow-hidden pointer-events-none opacity-30 ${className}`}>
      {Array.from({ length: columns }).map((_, i) => (
        <motion.div
          key={i}
          className="absolute top-0 flex flex-col font-mono text-xs"
          style={{
            left: `${(i / columns) * 100}%`,
            color,
            textShadow: `0 0 10px ${color}`,
          }}
          animate={{ y: ['-100%', '100%'] }}
          transition={{
            duration: speeds[speed] + Math.random() * 5,
            repeat: Infinity,
            ease: 'linear',
            delay: Math.random() * 5,
          }}
        >
          {Array.from({ length: 30 }).map((_, j) => (
            <span key={j} style={{ opacity: j / 30 }}>
              {chars[Math.floor(Math.random() * chars.length)]}
            </span>
          ))}
        </motion.div>
      ))}
    </div>
  );
};
