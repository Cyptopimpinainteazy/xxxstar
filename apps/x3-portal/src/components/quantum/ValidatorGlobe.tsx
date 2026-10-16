'use client';

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface ValidatorNode {
  id: string;
  lat: number;
  lng: number;
  city: string;
  country: string;
  status: 'online' | 'syncing' | 'offline';
  score: number;
  blocks: number;
  uptime: number;
  connected: boolean;
}

interface ConnectionLine {
  id: string;
  from: ValidatorNode;
  to: ValidatorNode;
  strength: number;
}

const CITIES: Array<{ city: string; country: string; lat: number; lng: number }> = [
  { city: 'New York', country: 'US', lat: 40.7128, lng: -74.0060 },
  { city: 'Los Angeles', country: 'US', lat: 34.0522, lng: -118.2437 },
  { city: 'London', country: 'UK', lat: 51.5074, lng: -0.1278 },
  { city: 'Tokyo', country: 'JP', lat: 35.6762, lng: 139.6503 },
  { city: 'Singapore', country: 'SG', lat: 1.3521, lng: 103.8198 },
  { city: 'Sydney', country: 'AU', lat: -33.8688, lng: 151.2093 },
  { city: 'Frankfurt', country: 'DE', lat: 50.1109, lng: 8.6821 },
  { city: 'São Paulo', country: 'BR', lat: -23.5505, lng: -46.6333 },
  { city: 'Mumbai', country: 'IN', lat: 19.0760, lng: 72.8777 },
  { city: 'Dubai', country: 'AE', lat: 25.2048, lng: 55.2708 },
  { city: 'Seoul', country: 'KR', lat: 37.5665, lng: 126.9780 },
  { city: 'Toronto', country: 'CA', lat: 43.6532, lng: -79.3832 },
  { city: 'Paris', country: 'FR', lat: 48.8566, lng: 2.3522 },
  { city: 'Amsterdam', country: 'NL', lat: 52.3676, lng: 4.9041 },
  { city: 'Hong Kong', country: 'HK', lat: 22.3193, lng: 114.1694 },
  { city: 'Johannesburg', country: 'ZA', lat: -26.2041, lng: 28.0473 },
  { city: 'Mexico City', country: 'MX', lat: 19.4326, lng: -99.1332 },
  { city: 'Stockholm', country: 'SE', lat: 59.3293, lng: 18.0686 },
  { city: 'Zurich', country: 'CH', lat: 47.3769, lng: 8.5417 },
  { city: 'Shanghai', country: 'CN', lat: 31.2304, lng: 121.4737 },
];

export default function ValidatorGlobe() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [nodes, setNodes] = useState<ValidatorNode[]>([]);
  const [connections, setConnections] = useState<ConnectionLine[]>([]);
  const [selectedNode, setSelectedNode] = useState<ValidatorNode | null>(null);
  const [globeRotation, setGlobeRotation] = useState(0);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  
  // Initialize nodes
  useEffect(() => {
    const initialNodes: ValidatorNode[] = CITIES.map((city, i) => ({
      id: `VAL-${city.city.substring(0, 3).toUpperCase()}-${String(i).padStart(3, '0')}`,
      lat: city.lat + (Math.random() - 0.5) * 5,
      lng: city.lng + (Math.random() - 0.5) * 5,
      city: city.city,
      country: city.country,
      status: Math.random() > 0.1 ? 'online' : Math.random() > 0.5 ? 'syncing' : 'offline',
      score: Math.floor(70 + Math.random() * 30),
      blocks: Math.floor(Math.random() * 50000),
      uptime: 95 + Math.random() * 5,
      connected: true
    }));
    
    setNodes(initialNodes);
    
    // Create connections
    const newConnections: ConnectionLine[] = [];
    initialNodes.forEach((node, i) => {
      const numConnections = Math.floor(Math.random() * 3) + 1;
      for (let j = 0; j < numConnections; j++) {
        const targetIdx = Math.floor(Math.random() * initialNodes.length);
        if (targetIdx !== i) {
          newConnections.push({
            id: `conn-${i}-${targetIdx}`,
            from: node,
            to: initialNodes[targetIdx],
            strength: Math.random()
          });
        }
      }
    });
    setConnections(newConnections);
  }, []);
  
  // Update nodes periodically
  useEffect(() => {
    const interval = setInterval(() => {
      setNodes(prev => prev.map(node => ({
        ...node,
        blocks: node.blocks + Math.floor(Math.random() * 10),
        score: Math.min(100, Math.max(60, node.score + (Math.random() - 0.5) * 5)),
        status: Math.random() > 0.98 
          ? (node.status === 'online' ? 'syncing' : 'online')
          : node.status
      })));
    }, 2000);
    
    return () => clearInterval(interval);
  }, []);
  
  // Auto-rotate globe
  useEffect(() => {
    if (isDragging) return;
    
    const interval = setInterval(() => {
      setGlobeRotation(prev => (prev + 0.2) % 360);
    }, 50);
    
    return () => clearInterval(interval);
  }, [isDragging]);
  
  // Canvas rendering for background
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
    
    // Grid lines
    const drawGrid = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      
      const centerX = canvas.offsetWidth / 2;
      const centerY = canvas.offsetHeight / 2;
      const radius = Math.min(centerX, centerY) * 0.85;
      
      // Draw earth glow
      const gradient = ctx.createRadialGradient(centerX, centerY, 0, centerX, centerY, radius * 1.2);
      gradient.addColorStop(0, 'rgba(0, 243, 255, 0.1)');
      gradient.addColorStop(0.5, 'rgba(0, 243, 255, 0.05)');
      gradient.addColorStop(1, 'transparent');
      
      ctx.beginPath();
      ctx.arc(centerX, centerY, radius * 1.2, 0, Math.PI * 2);
      ctx.fillStyle = gradient;
      ctx.fill();
      
      // Draw globe outline
      ctx.beginPath();
      ctx.arc(centerX, centerY, radius, 0, Math.PI * 2);
      ctx.strokeStyle = 'rgba(0, 243, 255, 0.3)';
      ctx.lineWidth = 2;
      ctx.stroke();
      
      // Draw latitude lines
      for (let lat = -60; lat <= 60; lat += 30) {
        const y = centerY - (lat / 90) * radius;
        const latRadius = Math.cos((lat * Math.PI) / 180) * radius;
        
        ctx.beginPath();
        ctx.ellipse(centerX, y, latRadius, latRadius * 0.3, 0, 0, Math.PI * 2);
        ctx.strokeStyle = 'rgba(0, 243, 255, 0.1)';
        ctx.lineWidth = 1;
        ctx.stroke();
      }
      
      // Draw longitude lines
      for (let lng = 0; lng < 180; lng += 30) {
        ctx.beginPath();
        ctx.save();
        ctx.translate(centerX, centerY);
        ctx.rotate(((lng + globeRotation) * Math.PI) / 180);
        ctx.ellipse(0, 0, radius * 0.1, radius, 0, 0, Math.PI * 2);
        ctx.restore();
        ctx.strokeStyle = 'rgba(0, 243, 255, 0.1)';
        ctx.lineWidth = 1;
        ctx.stroke();
      }
    };
    
    let animationId: number;
    const animate = () => {
      drawGrid();
      animationId = requestAnimationFrame(animate);
    };
    
    animate();
    
    return () => {
      window.removeEventListener('resize', resize);
      cancelAnimationFrame(animationId);
    };
  }, [globeRotation]);
  
  // Convert lat/lng to screen coordinates
  const latLngToScreen = useCallback((lat: number, lng: number) => {
    const adjustedLng = ((lng + globeRotation) % 360) - 180;
    
    // Check if point is on visible side of globe
    const visible = Math.abs(adjustedLng) < 90;
    
    // Project to screen coordinates
    const x = 50 + (adjustedLng / 180) * 40;
    const y = 50 - (lat / 90) * 40;
    
    return { x, y, visible };
  }, [globeRotation]);
  
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online': return '#00ff9d';
      case 'syncing': return '#ffff00';
      case 'offline': return '#ff4444';
      default: return '#888888';
    }
  };
  
  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true);
    setDragStart({ x: e.clientX, y: e.clientY });
  };
  
  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return;
    const delta = e.clientX - dragStart.x;
    setGlobeRotation(prev => (prev - delta * 0.5) % 360);
    setDragStart({ x: e.clientX, y: e.clientY });
  };
  
  const handleMouseUp = () => {
    setIsDragging(false);
  };
  
  return (
    <div 
      className="relative w-full h-[700px] bg-gradient-radial from-slate-900 via-slate-950 to-black rounded-3xl overflow-hidden cursor-grab active:cursor-grabbing"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    >
      {/* Background Canvas */}
      <canvas ref={canvasRef} className="absolute inset-0 w-full h-full" />
      
      {/* Connection Lines */}
      <svg className="absolute inset-0 w-full h-full pointer-events-none">
        {connections.map(conn => {
          const from = latLngToScreen(conn.from.lat, conn.from.lng);
          const to = latLngToScreen(conn.to.lat, conn.to.lng);
          
          if (!from.visible || !to.visible) return null;
          
          return (
            <motion.line
              key={conn.id}
              x1={`${from.x}%`}
              y1={`${from.y}%`}
              x2={`${to.x}%`}
              y2={`${to.y}%`}
              stroke="rgba(0, 243, 255, 0.2)"
              strokeWidth="1"
              initial={{ pathLength: 0 }}
              animate={{ pathLength: 1 }}
              transition={{ duration: 2, repeat: Infinity }}
            />
          );
        })}
      </svg>
      
      {/* Validator Nodes */}
      {nodes.map(node => {
        const pos = latLngToScreen(node.lat, node.lng);
        if (!pos.visible) return null;
        
        return (
          <motion.div
            key={node.id}
            className="absolute transform -translate-x-1/2 -translate-y-1/2 cursor-pointer"
            style={{ left: `${pos.x}%`, top: `${pos.y}%` }}
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            whileHover={{ scale: 1.5 }}
            onClick={() => setSelectedNode(node)}
          >
            {/* Node Glow */}
            <div 
              className="absolute inset-0 rounded-full blur-lg animate-pulse"
              style={{ 
                backgroundColor: getStatusColor(node.status),
                transform: 'scale(2)',
                opacity: 0.3
              }}
            />
            
            {/* Node Point */}
            <div 
              className="w-4 h-4 rounded-full border-2 relative"
              style={{ 
                backgroundColor: getStatusColor(node.status),
                borderColor: 'white',
                boxShadow: `0 0 20px ${getStatusColor(node.status)}`
              }}
            >
              {/* Pulse Ring */}
              <div 
                className="absolute inset-0 rounded-full animate-ping"
                style={{ 
                  backgroundColor: getStatusColor(node.status),
                  opacity: 0.5
                }}
              />
            </div>
          </motion.div>
        );
      })}
      
      {/* Selected Node Info Panel */}
      <AnimatePresence>
        {selectedNode && (
          <motion.div
            initial={{ opacity: 0, x: 50 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 50 }}
            className="absolute top-4 right-4 w-80 bg-slate-900/95 backdrop-blur-xl border border-cyan-500/30 rounded-2xl p-6 shadow-2xl shadow-cyan-500/20"
          >
            <button 
              className="absolute top-4 right-4 text-gray-500 hover:text-white"
              onClick={() => setSelectedNode(null)}
            >
              ✕
            </button>
            
            <div className="flex items-center gap-3 mb-4">
              <div 
                className="w-4 h-4 rounded-full"
                style={{ backgroundColor: getStatusColor(selectedNode.status) }}
              />
              <h3 className="font-orbitron font-bold text-white">{selectedNode.id}</h3>
            </div>
            
            <div className="space-y-3">
              <div className="flex justify-between">
                <span className="text-gray-400">Location</span>
                <span className="text-white font-mono">{selectedNode.city}, {selectedNode.country}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className={`font-mono uppercase ${
                  selectedNode.status === 'online' ? 'text-green-400' :
                  selectedNode.status === 'syncing' ? 'text-yellow-400' : 'text-red-400'
                }`}>{selectedNode.status}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Score</span>
                <div className="flex items-center gap-2">
                  <div className="w-20 h-2 bg-slate-700 rounded-full overflow-hidden">
                    <div 
                      className="h-full bg-gradient-to-r from-cyan-500 to-green-500"
                      style={{ width: `${selectedNode.score}%` }}
                    />
                  </div>
                  <span className="text-cyan-400 font-mono">{selectedNode.score}</span>
                </div>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Blocks Validated</span>
                <span className="text-white font-mono">{selectedNode.blocks.toLocaleString()}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Uptime</span>
                <span className="text-green-400 font-mono">{selectedNode.uptime.toFixed(2)}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Coordinates</span>
                <span className="text-gray-300 font-mono text-sm">
                  {selectedNode.lat.toFixed(4)}, {selectedNode.lng.toFixed(4)}
                </span>
              </div>
            </div>
            
            <div className="mt-6 pt-4 border-t border-slate-700">
              <button className="w-full py-2 bg-cyan-500/20 border border-cyan-500/30 rounded-lg text-cyan-400 font-mono text-sm hover:bg-cyan-500/30 transition-colors">
                VIEW FULL METRICS →
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
      
      {/* Stats Overlay */}
      <div className="absolute top-4 left-4 bg-slate-900/80 backdrop-blur-xl border border-cyan-500/20 rounded-xl p-4">
        <div className="flex items-center gap-2 mb-2">
          <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
          <span className="text-gray-400 text-sm">LIVE NETWORK</span>
        </div>
        <div className="text-3xl font-orbitron font-bold text-cyan-400">
          {nodes.filter(n => n.status === 'online').length}
        </div>
        <div className="text-xs text-gray-500 uppercase tracking-widest">
          Validators Online
        </div>
      </div>
      
      {/* Legend */}
      <div className="absolute bottom-4 left-4 flex gap-6">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-green-400" />
          <span className="text-gray-400 text-sm">Online</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-yellow-400" />
          <span className="text-gray-400 text-sm">Syncing</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-red-400" />
          <span className="text-gray-400 text-sm">Offline</span>
        </div>
      </div>
      
      {/* Instructions */}
      <div className="absolute bottom-4 right-4 text-gray-500 text-sm">
        Drag to rotate • Click node for details
      </div>
    </div>
  );
}
