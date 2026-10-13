'use client';

import React, { useEffect, useState, useRef, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface ThreatEvent {
  id: string;
  type: 'ddos' | 'sybil' | 'eclipse' | 'byzantine' | 'spam' | 'anomaly';
  severity: 'low' | 'medium' | 'high' | 'critical';
  source: string;
  target: string;
  timestamp: Date;
  details: string;
  confidence: number;
  mitigated: boolean;
}

interface NetworkMetrics {
  packetLoss: number;
  latency: number;
  bandwidth: number;
  connectionCount: number;
  anomalyScore: number;
}

const THREAT_TYPES = {
  ddos: { icon: '🎯', color: 'red', label: 'DDoS Attack' },
  sybil: { icon: '👥', color: 'orange', label: 'Sybil Attack' },
  eclipse: { icon: '🌑', color: 'purple', label: 'Eclipse Attack' },
  byzantine: { icon: '🏛️', color: 'yellow', label: 'Byzantine Fault' },
  spam: { icon: '📨', color: 'blue', label: 'Spam Flood' },
  anomaly: { icon: '⚠️', color: 'gray', label: 'Anomaly Detected' }
};

const SEVERITY_COLORS = {
  low: 'text-green-400 bg-green-500/20 border-green-500/30',
  medium: 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30',
  high: 'text-orange-400 bg-orange-500/20 border-orange-500/30',
  critical: 'text-red-400 bg-red-500/20 border-red-500/30'
};

export default function NeuralThreatDetector() {
  const [threats, setThreats] = useState<ThreatEvent[]>([]);
  const [metrics, setMetrics] = useState<NetworkMetrics>({
    packetLoss: 0.02,
    latency: 45,
    bandwidth: 89,
    connectionCount: 142,
    anomalyScore: 0.12
  });
  const [isScanning, setIsScanning] = useState(true);
  const [selectedThreat, setSelectedThreat] = useState<ThreatEvent | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  
  // Generate random threat event
  const generateThreat = useCallback((): ThreatEvent => {
    const types = Object.keys(THREAT_TYPES) as ThreatEvent['type'][];
    const severities: ThreatEvent['severity'][] = ['low', 'medium', 'high', 'critical'];
    const sources = ['192.168.1.', '10.0.0.', '172.16.0.', '203.45.67.'];
    
    return {
      id: `THR-${Date.now()}`,
      type: types[Math.floor(Math.random() * types.length)],
      severity: Math.random() > 0.7 
        ? severities[Math.floor(Math.random() * 2)]  // Usually low/medium
        : severities[Math.floor(Math.random() * severities.length)],
      source: `${sources[Math.floor(Math.random() * sources.length)]}${Math.floor(Math.random() * 255)}`,
      target: `VAL-${['NYC', 'TOK', 'SGP', 'FRA'][Math.floor(Math.random() * 4)]}-${String(Math.floor(Math.random() * 100)).padStart(3, '0')}`,
      timestamp: new Date(),
      details: [
        'Unusual packet pattern detected',
        'Connection surge from single source',
        'Validator isolation attempt',
        'Consensus message anomaly',
        'Rate limit exceeded',
        'Invalid signature sequence'
      ][Math.floor(Math.random() * 6)],
      confidence: 0.7 + Math.random() * 0.3,
      mitigated: Math.random() > 0.3
    };
  }, []);
  
  // Periodically add threats and update metrics
  useEffect(() => {
    if (!isScanning) return;
    
    const threatInterval = setInterval(() => {
      if (Math.random() > 0.7) {
        setThreats(prev => [generateThreat(), ...prev.slice(0, 49)]);
      }
    }, 3000);
    
    const metricsInterval = setInterval(() => {
      setMetrics(prev => ({
        packetLoss: Math.max(0, Math.min(0.1, prev.packetLoss + (Math.random() - 0.5) * 0.02)),
        latency: Math.max(20, Math.min(200, prev.latency + (Math.random() - 0.5) * 20)),
        bandwidth: Math.max(50, Math.min(100, prev.bandwidth + (Math.random() - 0.5) * 5)),
        connectionCount: Math.max(100, Math.min(200, prev.connectionCount + Math.floor((Math.random() - 0.5) * 10))),
        anomalyScore: Math.max(0, Math.min(1, prev.anomalyScore + (Math.random() - 0.5) * 0.1))
      }));
    }, 1000);
    
    return () => {
      clearInterval(threatInterval);
      clearInterval(metricsInterval);
    };
  }, [isScanning, generateThreat]);
  
  // Neural network visualization
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    
    const resize = () => {
      canvas.width = canvas.offsetWidth * 2;
      canvas.height = canvas.offsetHeight * 2;
      ctx.scale(2, 2);
    };
    
    resize();
    window.addEventListener('resize', resize);
    
    // Neural network nodes
    const layers = [5, 8, 10, 8, 5, 3];
    const nodes: Array<{ x: number; y: number; layer: number; activation: number }> = [];
    
    layers.forEach((count, layerIdx) => {
      for (let i = 0; i < count; i++) {
        const x = 50 + (layerIdx / (layers.length - 1)) * (canvas.offsetWidth - 100);
        const y = (canvas.offsetHeight / (count + 1)) * (i + 1);
        nodes.push({ x, y, layer: layerIdx, activation: Math.random() });
      }
    });
    
    let frame = 0;
    
    const animate = () => {
      ctx.clearRect(0, 0, canvas.offsetWidth, canvas.offsetHeight);
      frame++;
      
      // Update activations
      if (frame % 10 === 0) {
        nodes.forEach(node => {
          node.activation = Math.random();
        });
      }
      
      // Draw connections
      nodes.forEach((node, i) => {
        const nextLayerNodes = nodes.filter(n => n.layer === node.layer + 1);
        
        nextLayerNodes.forEach(target => {
          const gradient = ctx.createLinearGradient(node.x, node.y, target.x, target.y);
          const intensity = (node.activation + target.activation) / 2;
          
          gradient.addColorStop(0, `rgba(0, 243, 255, ${intensity * 0.3})`);
          gradient.addColorStop(1, `rgba(168, 85, 247, ${intensity * 0.3})`);
          
          ctx.beginPath();
          ctx.moveTo(node.x, node.y);
          ctx.lineTo(target.x, target.y);
          ctx.strokeStyle = gradient;
          ctx.lineWidth = intensity * 2;
          ctx.stroke();
        });
      });
      
      // Draw nodes
      nodes.forEach(node => {
        const pulse = Math.sin(frame * 0.05 + node.x + node.y) * 0.3 + 0.7;
        const radius = 6 + node.activation * 4;
        
        // Glow
        const glow = ctx.createRadialGradient(node.x, node.y, 0, node.x, node.y, radius * 3);
        glow.addColorStop(0, `rgba(0, 243, 255, ${node.activation * pulse * 0.5})`);
        glow.addColorStop(1, 'transparent');
        
        ctx.beginPath();
        ctx.arc(node.x, node.y, radius * 3, 0, Math.PI * 2);
        ctx.fillStyle = glow;
        ctx.fill();
        
        // Node
        ctx.beginPath();
        ctx.arc(node.x, node.y, radius, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(0, 243, 255, ${0.5 + node.activation * 0.5})`;
        ctx.fill();
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.lineWidth = 1;
        ctx.stroke();
      });
      
      requestAnimationFrame(animate);
    };
    
    const animationId = requestAnimationFrame(animate);
    
    return () => {
      window.removeEventListener('resize', resize);
      cancelAnimationFrame(animationId);
    };
  }, []);
  
  const getAnomalyLevel = (score: number): { label: string; color: string } => {
    if (score < 0.2) return { label: 'NORMAL', color: 'text-green-400' };
    if (score < 0.4) return { label: 'ELEVATED', color: 'text-yellow-400' };
    if (score < 0.6) return { label: 'HIGH', color: 'text-orange-400' };
    return { label: 'CRITICAL', color: 'text-red-400' };
  };
  
  const anomalyLevel = getAnomalyLevel(metrics.anomalyScore);
  
  return (
    <div className="bg-gradient-to-br from-slate-900/90 to-slate-800/50 rounded-3xl border border-cyan-500/30 backdrop-blur-xl overflow-hidden">
      {/* Header */}
      <div className="p-6 border-b border-cyan-500/20 bg-gradient-to-r from-slate-900 to-slate-800">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-2xl">🧠</span>
            <h3 className="text-xl font-orbitron text-cyan-400">NEURAL THREAT DETECTOR</h3>
            <div className={`w-2 h-2 rounded-full ${isScanning ? 'bg-green-400 animate-pulse' : 'bg-red-400'}`} />
          </div>
          <div className="flex items-center gap-4">
            <span className={`font-orbitron text-sm ${anomalyLevel.color}`}>
              THREAT LEVEL: {anomalyLevel.label}
            </span>
            <button
              onClick={() => setIsScanning(!isScanning)}
              className={`px-4 py-2 rounded-lg font-mono text-sm transition-colors ${
                isScanning 
                  ? 'bg-red-500/20 border border-red-500/30 text-red-400 hover:bg-red-500/30'
                  : 'bg-green-500/20 border border-green-500/30 text-green-400 hover:bg-green-500/30'
              }`}
            >
              {isScanning ? '⏸️ PAUSE' : '▶️ SCAN'}
            </button>
          </div>
        </div>
      </div>
      
      {/* Neural Network Visualization */}
      <div className="relative h-[200px] bg-gradient-to-br from-slate-950 to-slate-900">
        <canvas ref={canvasRef} className="absolute inset-0 w-full h-full" />
        
        {/* Overlay labels */}
        <div className="absolute inset-0 flex justify-between items-end px-8 pb-4 text-xs text-gray-500 font-mono">
          <span>INPUT</span>
          <span>HIDDEN LAYERS</span>
          <span>OUTPUT</span>
        </div>
        
        {/* Anomaly score overlay */}
        <div className="absolute top-4 right-4 bg-slate-900/90 rounded-lg p-4 border border-cyan-500/20">
          <div className="text-xs text-gray-500 mb-1">ANOMALY SCORE</div>
          <div className="flex items-center gap-2">
            <div className="w-24 h-3 bg-slate-700 rounded-full overflow-hidden">
              <motion.div
                className={`h-full ${
                  metrics.anomalyScore < 0.3 ? 'bg-green-500' :
                  metrics.anomalyScore < 0.6 ? 'bg-yellow-500' : 'bg-red-500'
                }`}
                animate={{ width: `${metrics.anomalyScore * 100}%` }}
                transition={{ duration: 0.5 }}
              />
            </div>
            <span className={`font-orbitron text-lg ${anomalyLevel.color}`}>
              {(metrics.anomalyScore * 100).toFixed(1)}%
            </span>
          </div>
        </div>
      </div>
      
      {/* Metrics Grid */}
      <div className="grid grid-cols-4 gap-4 p-4 border-b border-cyan-500/20">
        <div className="text-center p-3 bg-slate-800/50 rounded-xl">
          <div className="text-2xl font-orbitron text-cyan-400">{(metrics.packetLoss * 100).toFixed(2)}%</div>
          <div className="text-xs text-gray-500 uppercase tracking-widest">Packet Loss</div>
        </div>
        <div className="text-center p-3 bg-slate-800/50 rounded-xl">
          <div className="text-2xl font-orbitron text-green-400">{Math.round(metrics.latency)}ms</div>
          <div className="text-xs text-gray-500 uppercase tracking-widest">Latency</div>
        </div>
        <div className="text-center p-3 bg-slate-800/50 rounded-xl">
          <div className="text-2xl font-orbitron text-purple-400">{metrics.bandwidth.toFixed(1)}%</div>
          <div className="text-xs text-gray-500 uppercase tracking-widest">Bandwidth</div>
        </div>
        <div className="text-center p-3 bg-slate-800/50 rounded-xl">
          <div className="text-2xl font-orbitron text-yellow-400">{metrics.connectionCount}</div>
          <div className="text-xs text-gray-500 uppercase tracking-widest">Connections</div>
        </div>
      </div>
      
      {/* Threat Feed */}
      <div className="max-h-[300px] overflow-y-auto">
        <div className="p-4 border-b border-cyan-500/20 bg-slate-900/50 sticky top-0 z-10">
          <div className="flex items-center justify-between">
            <span className="text-gray-400 font-mono text-sm">LIVE THREAT FEED</span>
            <span className="text-gray-500 text-sm">{threats.length} events</span>
          </div>
        </div>
        
        <AnimatePresence mode="popLayout">
          {threats.length === 0 ? (
            <div className="p-8 text-center text-gray-500">
              <span className="text-4xl">🛡️</span>
              <p className="mt-2">No threats detected. Network is secure.</p>
            </div>
          ) : (
            threats.map((threat) => (
              <motion.div
                key={threat.id}
                initial={{ opacity: 0, x: -50 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: 50 }}
                className={`p-4 border-b border-slate-700/50 hover:bg-slate-800/50 cursor-pointer transition-colors ${
                  selectedThreat?.id === threat.id ? 'bg-slate-800/80' : ''
                }`}
                onClick={() => setSelectedThreat(threat.id === selectedThreat?.id ? null : threat)}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className="text-2xl">{THREAT_TYPES[threat.type].icon}</span>
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="text-white font-mono">{threat.id}</span>
                        <span className={`px-2 py-0.5 rounded text-xs border ${SEVERITY_COLORS[threat.severity]}`}>
                          {threat.severity.toUpperCase()}
                        </span>
                        {threat.mitigated && (
                          <span className="px-2 py-0.5 rounded text-xs bg-green-500/20 text-green-400 border border-green-500/30">
                            ✓ MITIGATED
                          </span>
                        )}
                      </div>
                      <div className="text-sm text-gray-400 mt-1">{THREAT_TYPES[threat.type].label}</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="font-mono text-cyan-400">{(threat.confidence * 100).toFixed(1)}%</div>
                    <div className="text-xs text-gray-500">confidence</div>
                  </div>
                </div>
                
                {/* Expanded details */}
                <AnimatePresence>
                  {selectedThreat?.id === threat.id && (
                    <motion.div
                      initial={{ height: 0, opacity: 0 }}
                      animate={{ height: 'auto', opacity: 1 }}
                      exit={{ height: 0, opacity: 0 }}
                      className="mt-4 pt-4 border-t border-slate-700/50"
                    >
                      <div className="grid grid-cols-2 gap-4 text-sm">
                        <div>
                          <div className="text-gray-500">Source IP</div>
                          <div className="font-mono text-red-400">{threat.source}</div>
                        </div>
                        <div>
                          <div className="text-gray-500">Target</div>
                          <div className="font-mono text-yellow-400">{threat.target}</div>
                        </div>
                        <div className="col-span-2">
                          <div className="text-gray-500">Details</div>
                          <div className="font-mono text-gray-300">{threat.details}</div>
                        </div>
                        <div className="col-span-2">
                          <div className="text-gray-500">Timestamp</div>
                          <div className="font-mono text-gray-300">
                            {threat.timestamp.toISOString()}
                          </div>
                        </div>
                      </div>
                      
                      {!threat.mitigated && (
                        <button 
                          className="mt-4 w-full py-2 bg-cyan-500/20 border border-cyan-500/30 rounded-lg text-cyan-400 font-mono text-sm hover:bg-cyan-500/30 transition-colors"
                          onClick={(e) => {
                            e.stopPropagation();
                            setThreats(prev => prev.map(t => 
                              t.id === threat.id ? { ...t, mitigated: true } : t
                            ));
                          }}
                        >
                          🛡️ INITIATE MITIGATION
                        </button>
                      )}
                    </motion.div>
                  )}
                </AnimatePresence>
              </motion.div>
            ))
          )}
        </AnimatePresence>
      </div>
      
      {/* Footer Stats */}
      <div className="p-4 border-t border-cyan-500/20 bg-slate-900/50 flex justify-around">
        <div className="text-center">
          <div className="font-orbitron text-green-400 text-lg">
            {threats.filter(t => t.mitigated).length}
          </div>
          <div className="text-xs text-gray-500">MITIGATED</div>
        </div>
        <div className="text-center">
          <div className="font-orbitron text-yellow-400 text-lg">
            {threats.filter(t => !t.mitigated && t.severity !== 'critical').length}
          </div>
          <div className="text-xs text-gray-500">ACTIVE</div>
        </div>
        <div className="text-center">
          <div className="font-orbitron text-red-400 text-lg">
            {threats.filter(t => t.severity === 'critical').length}
          </div>
          <div className="text-xs text-gray-500">CRITICAL</div>
        </div>
        <div className="text-center">
          <div className="font-orbitron text-cyan-400 text-lg">99.7%</div>
          <div className="text-xs text-gray-500">ACCURACY</div>
        </div>
      </div>
    </div>
  );
}
