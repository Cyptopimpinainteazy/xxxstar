'use client';

import React, { useState, useEffect, useRef, useMemo } from 'react';
import { motion, AnimatePresence, useMotionValue, useTransform } from 'framer-motion';

interface DataStreamVisualizerProps {
  className?: string;
  mode?: 'binary' | 'hex' | 'blocks' | 'neural';
  speed?: number;
  color?: string;
}

/**
 * DataStreamVisualizer - Real-time data stream visualization
 */
export const DataStreamVisualizer: React.FC<DataStreamVisualizerProps> = ({
  className = '',
  mode = 'binary',
  speed = 1,
  color = '#00ffea',
}) => {
  const [streams, setStreams] = useState<Array<{ id: number; data: string[]; x: number }>>([]);
  const containerRef = useRef<HTMLDivElement>(null);

  const generateData = () => {
    switch (mode) {
      case 'binary':
        return [...Array(20)].map(() => Math.random() > 0.5 ? '1' : '0');
      case 'hex':
        return [...Array(10)].map(() => Math.floor(Math.random() * 16).toString(16).toUpperCase());
      case 'blocks':
        return [...Array(5)].map(() => '█');
      case 'neural':
        return [...Array(8)].map(() => ['◯', '◉', '●'][Math.floor(Math.random() * 3)]);
      default:
        return ['0', '1'];
    }
  };

  useEffect(() => {
    const numStreams = mode === 'blocks' ? 15 : 25;
    const initialStreams = [...Array(numStreams)].map((_, i) => ({
      id: i,
      data: generateData(),
      x: (i / numStreams) * 100,
    }));
    setStreams(initialStreams);

    const interval = setInterval(() => {
      setStreams(prev => prev.map(stream => ({
        ...stream,
        data: generateData(),
      })));
    }, 100 / speed);

    return () => clearInterval(interval);
  }, [mode, speed]);

  return (
    <div
      ref={containerRef}
      className={`data-stream-visualizer ${className}`}
      style={{
        position: 'relative',
        width: '100%',
        height: '300px',
        overflow: 'hidden',
        background: '#000',
        fontFamily: "'MS Gothic', monospace",
        fontSize: mode === 'blocks' ? '20px' : '12px',
      }}
    >
      {streams.map((stream) => (
        <motion.div
          key={stream.id}
          style={{
            position: 'absolute',
            left: `${stream.x}%`,
            top: 0,
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'space-around',
            color,
            textShadow: `0 0 5px ${color}`,
          }}
          initial={{ opacity: 0 }}
          animate={{ opacity: [0.3, 0.8, 0.3] }}
          transition={{ duration: 1 / speed, repeat: Infinity }}
        >
          {stream.data.map((char, i) => (
            <motion.span
              key={i}
              animate={{ opacity: [0.5, 1, 0.5] }}
              transition={{
                duration: 0.5 / speed,
                delay: i * 0.05,
                repeat: Infinity,
              }}
            >
              {char}
            </motion.span>
          ))}
        </motion.div>
      ))}

      {/* Scan line effect */}
      <motion.div
        style={{
          position: 'absolute',
          left: 0,
          right: 0,
          height: '2px',
          background: `linear-gradient(90deg, transparent, ${color}, transparent)`,
          boxShadow: `0 0 10px ${color}`,
        }}
        animate={{ top: ['0%', '100%'] }}
        transition={{ duration: 2 / speed, repeat: Infinity, ease: 'linear' }}
      />
    </div>
  );
};

/**
 * WaveformVisualizer - Audio-style waveform visualization
 */
export const WaveformVisualizer: React.FC<{
  bars?: number;
  color?: string;
  className?: string;
  mode?: 'bars' | 'wave' | 'circular';
}> = ({
  bars = 64,
  color = '#00ffea',
  className = '',
  mode = 'bars',
}) => {
  const [levels, setLevels] = useState<number[]>([]);

  useEffect(() => {
    const generateLevels = () => {
      return [...Array(bars)].map((_, i) => {
        const base = Math.sin(i / bars * Math.PI * 4) * 0.3 + 0.5;
        return base + Math.random() * 0.4;
      });
    };

    setLevels(generateLevels());
    const interval = setInterval(() => setLevels(generateLevels()), 50);
    return () => clearInterval(interval);
  }, [bars]);

  if (mode === 'circular') {
    return (
      <div
        className={`waveform-visualizer circular ${className}`}
        style={{
          width: '300px',
          height: '300px',
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        {levels.map((level, i) => {
          const angle = (i / bars) * 360;
          const height = 20 + level * 80;
          return (
            <motion.div
              key={i}
              style={{
                position: 'absolute',
                width: '3px',
                background: `linear-gradient(180deg, ${color}, ${color}00)`,
                borderRadius: '2px',
                transformOrigin: 'bottom center',
                transform: `rotate(${angle}deg) translateY(-60px)`,
                boxShadow: `0 0 10px ${color}88`,
              }}
              animate={{ height }}
              transition={{ duration: 0.05 }}
            />
          );
        })}
        <div
          style={{
            width: '100px',
            height: '100px',
            borderRadius: '50%',
            background: `radial-gradient(circle, ${color}22, transparent)`,
            boxShadow: `0 0 40px ${color}44`,
          }}
        />
      </div>
    );
  }

  return (
    <div
      className={`waveform-visualizer ${className}`}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '2px',
        height: '100px',
        padding: '0 1rem',
      }}
    >
      {levels.map((level, i) => (
        <motion.div
          key={i}
          style={{
            width: `${100 / bars}%`,
            maxWidth: '8px',
            background: mode === 'wave'
              ? `linear-gradient(180deg, ${color}00, ${color}, ${color}00)`
              : color,
            borderRadius: '2px',
            boxShadow: `0 0 10px ${color}66`,
          }}
          animate={{ height: `${level * 100}%` }}
          transition={{ duration: 0.05 }}
        />
      ))}
    </div>
  );
};

/**
 * BlockchainFlowVisualizer - Animated blockchain data flow
 */
export const BlockchainFlowVisualizer: React.FC<{
  className?: string;
}> = ({ className = '' }) => {
  const [blocks, setBlocks] = useState<Array<{ id: number; hash: string; status: 'pending' | 'confirmed' | 'finalized' }>>([]);

  useEffect(() => {
    const addBlock = () => {
      const newBlock = {
        id: Date.now(),
        hash: `0x${Math.random().toString(16).slice(2, 10)}`,
        status: 'pending' as const,
      };
      setBlocks(prev => [...prev.slice(-6), newBlock]);
    };

    addBlock();
    const interval = setInterval(addBlock, 2000);

    // Status progression
    const statusInterval = setInterval(() => {
      setBlocks(prev => prev.map(block => ({
        ...block,
        status: block.status === 'pending' ? 'confirmed' : block.status === 'confirmed' ? 'finalized' : block.status,
      })));
    }, 1000);

    return () => {
      clearInterval(interval);
      clearInterval(statusInterval);
    };
  }, []);

  const statusColors = {
    pending: '#ffd93d',
    confirmed: '#00ffea',
    finalized: '#00ff88',
  };

  return (
    <div className={`blockchain-flow-visualizer ${className}`} style={{
      display: 'flex',
      alignItems: 'center',
      gap: '1rem',
      padding: '2rem',
      overflow: 'hidden',
    }}>
      <AnimatePresence mode="popLayout">
        {blocks.map((block, i) => (
          <motion.div
            key={block.id}
            initial={{ opacity: 0, x: 100, scale: 0.8 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: -100, scale: 0.8 }}
            transition={{ duration: 0.5 }}
            style={{
              padding: '1rem',
              background: `linear-gradient(135deg, #1a1a2e, #0a0a1a)`,
              border: `2px solid ${statusColors[block.status]}`,
              borderRadius: '8px',
              minWidth: '120px',
              textAlign: 'center',
              boxShadow: `0 0 20px ${statusColors[block.status]}44`,
            }}
          >
            <div style={{
              fontSize: '10px',
              color: '#666',
              fontFamily: 'monospace',
            }}>
              BLOCK #{i + 1}
            </div>
            <div style={{
              fontSize: '12px',
              color: statusColors[block.status],
              fontFamily: 'monospace',
              marginTop: '4px',
            }}>
              {block.hash}
            </div>
            <div style={{
              fontSize: '9px',
              color: statusColors[block.status],
              textTransform: 'uppercase',
              marginTop: '8px',
              fontWeight: 'bold',
            }}>
              {block.status}
            </div>
          </motion.div>
        ))}
      </AnimatePresence>

      {/* Connection lines */}
      <svg style={{
        position: 'absolute',
        width: '100%',
        height: '2px',
        top: '50%',
        left: 0,
        pointerEvents: 'none',
      }}>
        <motion.line
          x1="0"
          y1="0"
          x2="100%"
          y2="0"
          stroke="#00ffea44"
          strokeWidth="2"
          strokeDasharray="10 5"
          animate={{
            strokeDashoffset: [0, -30],
          }}
          transition={{
            duration: 1,
            repeat: Infinity,
            ease: 'linear',
          }}
        />
      </svg>
    </div>
  );
};

/**
 * DataVisualizersShowcase - Display all data visualizers
 */
export const DataVisualizersShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`data-visualizers-showcase ${className}`} style={{
      background: 'linear-gradient(180deg, #0a0a1a 0%, #000 100%)',
      padding: '2rem',
      borderRadius: '1rem',
    }}>
      <div className="text-center mb-8">
        <h2 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-green-400">
          DATA STREAMS
        </h2>
        <p className="text-gray-400 mt-2">Real-time data visualization</p>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem' }}>
        {/* Binary stream */}
        <div>
          <p style={{ color: '#00ffea', fontFamily: 'monospace', marginBottom: '0.5rem', fontSize: '12px' }}>
            BINARY STREAM
          </p>
          <DataStreamVisualizer mode="binary" speed={1.5} />
        </div>

        {/* Waveform */}
        <div>
          <p style={{ color: '#ff00ff', fontFamily: 'monospace', marginBottom: '0.5rem', fontSize: '12px' }}>
            QUANTUM WAVEFORM
          </p>
          <div style={{ background: '#0a0a0a', borderRadius: '8px', padding: '1rem' }}>
            <WaveformVisualizer bars={64} color="#ff00ff" />
          </div>
        </div>

        {/* Circular waveform */}
        <div style={{ display: 'flex', justifyContent: 'center' }}>
          <WaveformVisualizer mode="circular" color="#ffd93d" />
        </div>

        {/* Blockchain flow */}
        <div>
          <p style={{ color: '#00ff88', fontFamily: 'monospace', marginBottom: '0.5rem', fontSize: '12px' }}>
            BLOCKCHAIN FLOW
          </p>
          <div style={{ background: '#0a0a0a', borderRadius: '8px', position: 'relative', overflow: 'hidden' }}>
            <BlockchainFlowVisualizer />
          </div>
        </div>
      </div>
    </div>
  );
};

export default DataStreamVisualizer;
