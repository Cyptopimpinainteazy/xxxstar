'use client';

import React, { useEffect, useState, useRef, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface MatrixRainProps {
  color?: string;
  speed?: number;
  density?: number;
  className?: string;
  children?: React.ReactNode;
  interactive?: boolean;
}

// Characters for matrix rain
const MATRIX_CHARS = 'ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ@#$%^&*()';

/**
 * MatrixRain - The iconic falling code effect from The Matrix
 */
export const MatrixRain: React.FC<MatrixRainProps> = ({
  color = '#00ff41',
  speed = 1,
  density = 0.8,
  className = '',
  children,
  interactive = true,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const [mousePos, setMousePos] = useState({ x: -1000, y: -1000 });

  useEffect(() => {
    const updateDimensions = () => {
      if (containerRef.current) {
        setDimensions({
          width: containerRef.current.offsetWidth,
          height: containerRef.current.offsetHeight,
        });
      }
    };
    
    updateDimensions();
    window.addEventListener('resize', updateDimensions);
    return () => window.removeEventListener('resize', updateDimensions);
  }, []);

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!interactive || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    setMousePos({
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
    });
  };

  const columns = useMemo(() => {
    const charWidth = 14;
    const numCols = Math.floor((dimensions.width / charWidth) * density);
    
    return [...Array(numCols)].map((_, i) => ({
      id: i,
      x: i * (charWidth / density),
      speed: 0.5 + Math.random() * speed,
      delay: Math.random() * 5,
      length: 10 + Math.floor(Math.random() * 20),
    }));
  }, [dimensions.width, density, speed]);

  return (
    <div
      ref={containerRef}
      className={`matrix-rain ${className}`}
      onMouseMove={handleMouseMove}
      style={{
        position: 'relative',
        width: '100%',
        minHeight: '500px',
        overflow: 'hidden',
        background: '#000',
      }}
    >
      {/* Matrix columns */}
      {columns.map((col) => (
        <MatrixColumn
          key={col.id}
          x={col.x}
          speed={col.speed}
          delay={col.delay}
          length={col.length}
          height={dimensions.height}
          color={color}
          mousePos={mousePos}
          interactive={interactive}
        />
      ))}

      {/* Overlay gradients */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          background: 'linear-gradient(180deg, rgba(0,0,0,0.8) 0%, transparent 20%, transparent 80%, rgba(0,0,0,0.8) 100%)',
          pointerEvents: 'none',
        }}
      />

      {/* Vignette */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          boxShadow: 'inset 0 0 100px rgba(0,0,0,0.8)',
          pointerEvents: 'none',
        }}
      />

      {/* Content */}
      <div style={{ position: 'relative', zIndex: 10 }}>
        {children}
      </div>
    </div>
  );
};

/**
 * Single matrix column with falling characters
 */
const MatrixColumn: React.FC<{
  x: number;
  speed: number;
  delay: number;
  length: number;
  height: number;
  color: string;
  mousePos: { x: number; y: number };
  interactive: boolean;
}> = ({ x, speed, delay, length, height, color, mousePos, interactive }) => {
  const [chars, setChars] = useState<string[]>([]);
  const [offset, setOffset] = useState(-length * 20);

  useEffect(() => {
    // Initialize characters
    setChars(
      [...Array(length)].map(() =>
        MATRIX_CHARS[Math.floor(Math.random() * MATRIX_CHARS.length)]
      )
    );
  }, [length]);

  useEffect(() => {
    let animationId: number;
    let lastTime = 0;

    const animate = (time: number) => {
      if (lastTime) {
        const delta = (time - lastTime) / 1000;
        setOffset((prev) => {
          const newOffset = prev + delta * speed * 100;
          return newOffset > height + length * 20 ? -length * 20 : newOffset;
        });

        // Randomly change characters
        if (Math.random() < 0.1) {
          setChars((prev) =>
            prev.map((char, i) =>
              Math.random() < 0.1
                ? MATRIX_CHARS[Math.floor(Math.random() * MATRIX_CHARS.length)]
                : char
            )
          );
        }
      }
      lastTime = time;
      animationId = requestAnimationFrame(animate);
    };

    const timeoutId = setTimeout(() => {
      animationId = requestAnimationFrame(animate);
    }, delay * 1000);

    return () => {
      clearTimeout(timeoutId);
      cancelAnimationFrame(animationId);
    };
  }, [speed, delay, height, length]);

  // Calculate distance from mouse for interactive glow
  const distFromMouse = interactive
    ? Math.sqrt(Math.pow(x - mousePos.x, 2) + Math.pow(offset - mousePos.y, 2))
    : 1000;
  const glowIntensity = interactive ? Math.max(0, 1 - distFromMouse / 150) : 0;

  return (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: offset,
        display: 'flex',
        flexDirection: 'column',
        fontFamily: "'MS Gothic', monospace",
        fontSize: '14px',
        lineHeight: '20px',
        color,
        filter: glowIntensity > 0 ? `brightness(${1 + glowIntensity * 2})` : undefined,
        textShadow: glowIntensity > 0 
          ? `0 0 ${10 + glowIntensity * 20}px ${color}, 0 0 ${20 + glowIntensity * 40}px ${color}`
          : `0 0 5px ${color}`,
      }}
    >
      {chars.map((char, i) => (
        <span
          key={i}
          style={{
            opacity: i === 0 ? 1 : (length - i) / length,
            color: i === 0 ? '#fff' : color,
            textShadow: i === 0 ? `0 0 10px #fff, 0 0 20px ${color}` : undefined,
          }}
        >
          {char}
        </span>
      ))}
    </div>
  );
};

/**
 * MatrixText - Text that reveals with matrix decoding effect
 */
export const MatrixText: React.FC<{
  text: string;
  className?: string;
  speed?: number;
  color?: string;
}> = ({ text, className = '', speed = 50, color = '#00ff41' }) => {
  const [displayText, setDisplayText] = useState<string[]>(
    text.split('').map(() => MATRIX_CHARS[Math.floor(Math.random() * MATRIX_CHARS.length)])
  );
  const [revealedCount, setRevealedCount] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setDisplayText((prev) =>
        prev.map((char, i) =>
          i < revealedCount
            ? text[i]
            : MATRIX_CHARS[Math.floor(Math.random() * MATRIX_CHARS.length)]
        )
      );
    }, 30);

    const revealInterval = setInterval(() => {
      setRevealedCount((prev) => {
        if (prev >= text.length) {
          clearInterval(revealInterval);
          return prev;
        }
        return prev + 1;
      });
    }, speed);

    return () => {
      clearInterval(interval);
      clearInterval(revealInterval);
    };
  }, [text, speed]);

  return (
    <span
      className={`matrix-text ${className}`}
      style={{
        fontFamily: "'MS Gothic', monospace",
        color,
        textShadow: `0 0 10px ${color}`,
      }}
    >
      {displayText.join('')}
    </span>
  );
};

/**
 * MatrixRainShowcase - Display matrix effects
 */
export const MatrixRainShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  const [key, setKey] = useState(0);

  return (
    <div className={`matrix-rain-showcase ${className}`}>
      <MatrixRain interactive={true}>
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            minHeight: '500px',
            padding: '2rem',
            textAlign: 'center',
          }}
        >
          <motion.h1
            key={key}
            style={{
              fontSize: 'clamp(2rem, 8vw, 6rem)',
              fontFamily: "'MS Gothic', 'Orbitron', monospace",
              fontWeight: 'bold',
              color: '#00ff41',
              textShadow: '0 0 20px #00ff41, 0 0 40px #00ff4188',
              marginBottom: '1rem',
            }}
          >
            <MatrixText text="WAKE UP, NEO" speed={100} />
          </motion.h1>

          <p
            style={{
              fontSize: '1.2rem',
              color: '#00ff41',
              fontFamily: 'monospace',
              textShadow: '0 0 10px #00ff41',
              maxWidth: '500px',
            }}
          >
            The Matrix has you... Follow the white rabbit. 
            <br />
            <span style={{ color: '#fff' }}>X3 Chain awaits.</span>
          </p>

          <motion.button
            onClick={() => setKey((k) => k + 1)}
            whileHover={{ scale: 1.05, boxShadow: '0 0 30px #00ff41' }}
            whileTap={{ scale: 0.95 }}
            style={{
              marginTop: '2rem',
              padding: '16px 32px',
              background: 'transparent',
              border: '2px solid #00ff41',
              color: '#00ff41',
              fontFamily: "'MS Gothic', monospace",
              fontSize: '1rem',
              cursor: 'pointer',
              boxShadow: '0 0 10px #00ff4144',
            }}
          >
            [ TAKE THE RED PILL ]
          </motion.button>
        </div>
      </MatrixRain>
    </div>
  );
};

export default MatrixRain;
