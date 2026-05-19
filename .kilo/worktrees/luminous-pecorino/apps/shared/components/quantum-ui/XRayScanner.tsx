'use client';

import React, { useState, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface XRayScannerProps {
  className?: string;
  scanColor?: string;
  mode?: 'body' | 'blockchain' | 'circuit' | 'neural';
}

/**
 * XRayScanner - Interactive X-ray scanner effect revealing skeleton/inner structure
 * Hover to scan and reveal the inner layer
 */
export const XRayScanner: React.FC<XRayScannerProps> = ({
  className = '',
  scanColor = '#00ffea',
  mode = 'blockchain',
}) => {
  const [scanPosition, setScanPosition] = useState(50);
  const [isScanning, setIsScanning] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const y = ((e.clientY - rect.top) / rect.height) * 100;
    setScanPosition(Math.max(0, Math.min(100, y)));
    setIsScanning(true);
  };

  const handleMouseLeave = () => {
    setIsScanning(false);
    setScanPosition(50);
  };

  const outerContent = mode === 'blockchain' ? (
    <BlockchainOuter />
  ) : mode === 'circuit' ? (
    <CircuitOuter />
  ) : mode === 'neural' ? (
    <NeuralOuter />
  ) : (
    <HumanOuter />
  );

  const innerContent = mode === 'blockchain' ? (
    <BlockchainInner />
  ) : mode === 'circuit' ? (
    <CircuitInner />
  ) : mode === 'neural' ? (
    <NeuralInner />
  ) : (
    <SkeletonInner />
  );

  return (
    <div
      ref={containerRef}
      className={`xray-scanner-container ${className}`}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      style={{
        position: 'relative',
        width: '100%',
        height: '500px',
        overflow: 'hidden',
        cursor: 'ns-resize',
        background: `radial-gradient(ellipse at 50% 50%, #fff 35%, ${scanColor})`,
        boxShadow: '0 0 50px 20px rgba(0,0,0,0.5) inset',
        borderRadius: '1rem',
      }}
    >
      {/* Outer Layer (visible) */}
      <div className="xray-outer-layer" style={{ position: 'absolute', inset: 0, zIndex: 1 }}>
        {outerContent}
      </div>

      {/* Inner Layer (revealed by scanner) */}
      <motion.div
        className="xray-inner-layer"
        style={{
          position: 'absolute',
          inset: 0,
          zIndex: 2,
          background: 'linear-gradient(90deg, #000, #333, #000)',
          clipPath: `polygon(0 ${scanPosition - 10}%, 100% ${scanPosition - 10}%, 100% ${scanPosition + 10}%, 0 ${scanPosition + 10}%)`,
        }}
        animate={{
          clipPath: isScanning 
            ? `polygon(0 ${scanPosition - 10}%, 100% ${scanPosition - 10}%, 100% ${scanPosition + 10}%, 0 ${scanPosition + 10}%)`
            : `polygon(0 45%, 100% 45%, 100% 55%, 0 55%)`,
        }}
        transition={{ duration: 0.15, ease: 'easeOut' }}
      >
        {innerContent}
      </motion.div>

      {/* Scanner Frame */}
      <motion.div
        className="scanner-frame"
        style={{
          position: 'absolute',
          left: '-5px',
          right: '-5px',
          height: '20%',
          border: `5px solid ${scanColor}33`,
          borderRadius: '8px',
          zIndex: 3,
          pointerEvents: 'none',
          boxShadow: `
            0 0 20px 5px ${scanColor},
            0 0 20px 10px ${scanColor}33 inset,
            0 0 5px 5px rgba(0,0,0,0.5) inset,
            0 0 30px 2px rgba(0,0,0,0.8)
          `,
          background: `
            linear-gradient(90deg, #0c0c0c, #383838, #0c0c0c),
            linear-gradient(90deg, #0c0c0c, #383838, #0c0c0c)
          `,
          backgroundSize: '100% 40%, 100% 40%',
          backgroundPosition: '50% -70%, 50% 170%',
          backgroundRepeat: 'no-repeat',
        }}
        animate={{
          top: `${scanPosition - 10}%`,
        }}
        transition={{ duration: 0.15, ease: 'easeOut' }}
      >
        {/* Scanner glow lines */}
        <div style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          height: '2px',
          background: `linear-gradient(90deg, transparent, ${scanColor}, transparent)`,
          boxShadow: `0 0 10px ${scanColor}`,
        }} />
        <div style={{
          position: 'absolute',
          bottom: 0,
          left: 0,
          right: 0,
          height: '2px',
          background: `linear-gradient(90deg, transparent, ${scanColor}, transparent)`,
          boxShadow: `0 0 10px ${scanColor}`,
        }} />
      </motion.div>

      {/* Control Buttons */}
      <div className="xray-controls" style={{
        position: 'absolute',
        bottom: '20px',
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        gap: '10px',
        zIndex: 10,
      }}>
        <motion.button
          whileHover={{ scale: 1.1, boxShadow: `0 0 20px ${scanColor}` }}
          whileTap={{ scale: 0.95 }}
          style={{
            padding: '8px 16px',
            background: '#111',
            border: `2px solid ${scanColor}33`,
            borderRadius: '8px',
            color: scanColor,
            fontSize: '12px',
            fontFamily: 'monospace',
            cursor: 'pointer',
          }}
          onClick={() => setScanPosition(0)}
        >
          SCAN TOP
        </motion.button>
        <motion.button
          whileHover={{ scale: 1.1, boxShadow: `0 0 20px ${scanColor}` }}
          whileTap={{ scale: 0.95 }}
          style={{
            padding: '8px 16px',
            background: '#111',
            border: `2px solid ${scanColor}33`,
            borderRadius: '8px',
            color: scanColor,
            fontSize: '12px',
            fontFamily: 'monospace',
            cursor: 'pointer',
          }}
          onClick={() => setScanPosition(100)}
        >
          SCAN BOTTOM
        </motion.button>
      </div>

      {/* Label */}
      <div style={{
        position: 'absolute',
        top: '10px',
        right: '10px',
        color: scanColor,
        fontSize: '10px',
        fontFamily: 'monospace',
        opacity: 0.7,
        zIndex: 10,
      }}>
        X3 X-RAY v2.0
      </div>
    </div>
  );
};

// Blockchain Outer - Visual blocks representation
const BlockchainOuter: React.FC = () => (
  <div className="blockchain-outer" style={{
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    padding: '40px',
    gap: '20px',
  }}>
    {[...Array(5)].map((_, i) => (
      <motion.div
        key={i}
        initial={{ opacity: 0, x: -50 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ delay: i * 0.1 }}
        style={{
          width: '80%',
          height: '60px',
          background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)',
          borderRadius: '12px',
          border: '2px solid rgba(0,255,234,0.2)',
          display: 'flex',
          alignItems: 'center',
          padding: '0 20px',
          gap: '15px',
          boxShadow: '0 4px 15px rgba(0,0,0,0.3)',
        }}
      >
        <div style={{
          width: '30px',
          height: '30px',
          borderRadius: '6px',
          background: 'linear-gradient(135deg, #00ffea, #0080ff)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: '#000',
          fontWeight: 'bold',
          fontSize: '14px',
        }}>
          {i + 1}
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ color: '#00ffea', fontSize: '12px', fontFamily: 'monospace' }}>
            BLOCK #{(1000000 + i * 1337).toString()}
          </div>
          <div style={{ color: '#666', fontSize: '10px', fontFamily: 'monospace' }}>
            0x{Math.random().toString(16).slice(2, 18)}...
          </div>
        </div>
        <div style={{
          width: '8px',
          height: '8px',
          borderRadius: '50%',
          background: '#00ff88',
          boxShadow: '0 0 10px #00ff88',
        }} />
      </motion.div>
    ))}
  </div>
);

// Blockchain Inner - Reveals the transaction data
const BlockchainInner: React.FC = () => (
  <div className="blockchain-inner" style={{
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    padding: '40px',
    gap: '8px',
    background: 'linear-gradient(180deg, #0a0a0a 0%, #1a1a1a 100%)',
  }}>
    {[...Array(12)].map((_, i) => (
      <div
        key={i}
        style={{
          width: '90%',
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          fontFamily: 'monospace',
          fontSize: '11px',
        }}
      >
        <span style={{ color: '#00ffea', minWidth: '80px' }}>
          TX_{(i * 7).toString().padStart(4, '0')}
        </span>
        <span style={{ color: '#fff', flex: 1, opacity: 0.8 }}>
          {Math.random().toString(16).slice(2, 42)}
        </span>
        <span style={{ color: i % 3 === 0 ? '#ff6b6b' : '#00ff88' }}>
          {i % 3 === 0 ? 'PENDING' : 'CONFIRMED'}
        </span>
      </div>
    ))}
    <div style={{
      position: 'absolute',
      bottom: '20px',
      color: '#00ffea',
      fontSize: '10px',
      fontFamily: 'monospace',
    }}>
      [ MERKLE TREE VISIBLE • DEPTH: 7 ]
    </div>
  </div>
);

// Circuit Outer
const CircuitOuter: React.FC = () => (
  <div style={{
    width: '100%',
    height: '100%',
    background: 'linear-gradient(135deg, #0a192f 0%, #172a45 100%)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  }}>
    <div style={{
      fontSize: '48px',
      color: '#00ffea',
      textShadow: '0 0 30px rgba(0,255,234,0.5)',
      fontFamily: 'Orbitron, monospace',
      fontWeight: 'bold',
    }}>
      X3 CPU
    </div>
  </div>
);

// Circuit Inner
const CircuitInner: React.FC = () => (
  <div style={{
    width: '100%',
    height: '100%',
    background: '#0a0a0a',
    backgroundImage: `
      linear-gradient(rgba(0,255,234,0.1) 1px, transparent 1px),
      linear-gradient(90deg, rgba(0,255,234,0.1) 1px, transparent 1px)
    `,
    backgroundSize: '20px 20px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  }}>
    <pre style={{
      color: '#00ff88',
      fontSize: '10px',
      fontFamily: 'monospace',
      textAlign: 'center',
    }}>
{`   ┌──────────────────────────┐
   │  QUANTUM CORE ARRAY     │
   │  ╔══════════════════╗   │
   │  ║ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ║   │
   │  ║ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ║   │
   │  ║ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ║   │
   │  ║ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ◉ ║   │
   │  ╚══════════════════╝   │
   │  STATUS: OPERATIONAL    │
   └──────────────────────────┘`}
    </pre>
  </div>
);

// Neural Outer
const NeuralOuter: React.FC = () => (
  <div style={{
    width: '100%',
    height: '100%',
    background: 'radial-gradient(circle at 50% 50%, #1a0a2e 0%, #0a0a1a 100%)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexDirection: 'column',
    gap: '20px',
  }}>
    <div style={{
      width: '150px',
      height: '150px',
      borderRadius: '50%',
      background: 'conic-gradient(from 0deg, #ff00ff, #00ffff, #ff00ff)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      animation: 'spin 10s linear infinite',
    }}>
      <div style={{
        width: '130px',
        height: '130px',
        borderRadius: '50%',
        background: '#0a0a1a',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: '#ff00ff',
        fontSize: '24px',
        fontWeight: 'bold',
      }}>
        AI
      </div>
    </div>
    <div style={{ color: '#ff00ff', fontFamily: 'monospace' }}>NEURAL INTERFACE</div>
  </div>
);

// Neural Inner
const NeuralInner: React.FC = () => (
  <div style={{
    width: '100%',
    height: '100%',
    background: '#0a0a0a',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexDirection: 'column',
  }}>
    <pre style={{
      color: '#ff00ff',
      fontSize: '8px',
      lineHeight: '10px',
      fontFamily: 'monospace',
    }}>
{`    ╭────────────────────────────╮
    │   ◯───◯───◯───◯───◯───◯   │
    │   │╲ │╲ │╲ │╲ │╲ │╲ │   │
    │   │ ╲│ ╲│ ╲│ ╲│ ╲│ ╲│   │
    │   ◯───◯───◯───◯───◯───◯   │
    │   │╲ │╲ │╲ │╲ │╲ │╲ │   │
    │   │ ╲│ ╲│ ╲│ ╲│ ╲│ ╲│   │
    │   ◯───◯───◯───◯───◯───◯   │
    │   │╲ │╲ │╲ │╲ │╲ │╲ │   │
    │   │ ╲│ ╲│ ╲│ ╲│ ╲│ ╲│   │
    │   ◯───◯───◯───◯───◯───◯   │
    ╰────────────────────────────╯
     LAYER 0  LAYER 1  LAYER 2`}
    </pre>
  </div>
);

// Human Outer (placeholder)
const HumanOuter: React.FC = () => (
  <div style={{
    width: '100%',
    height: '100%',
    background: 'linear-gradient(180deg, #f0f0f0 0%, #e0e0e0 100%)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '100px',
  }}>
    🧑
  </div>
);

// Skeleton Inner (placeholder)
const SkeletonInner: React.FC = () => (
  <div style={{
    width: '100%',
    height: '100%',
    background: '#1a1a1a',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '100px',
  }}>
    💀
  </div>
);

/**
 * XRayScannerShowcase - Display all scanner modes
 */
export const XRayScannerShowcase: React.FC<{ className?: string }> = ({ className = '' }) => {
  return (
    <div className={`xray-scanner-showcase ${className}`}>
      <div className="text-center mb-8">
        <h2 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-purple-500">
          X-RAY SCANNER
        </h2>
        <p className="text-gray-400 mt-2">Hover to reveal inner structure</p>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        <div>
          <XRayScanner mode="blockchain" scanColor="#00ffea" />
          <p className="text-center text-cyan-400 mt-4 font-mono">BLOCKCHAIN MODE</p>
        </div>
        <div>
          <XRayScanner mode="neural" scanColor="#ff00ff" />
          <p className="text-center text-purple-400 mt-4 font-mono">NEURAL MODE</p>
        </div>
      </div>
    </div>
  );
};

export default XRayScanner;
