'use client';

import { useEffect, useRef, useState } from 'react';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';

interface GpuNode {
  id: string;
  lat: number;
  lng: number;
  city: string;
  country: string;
  gpu: string;
  vram: number;
  status: 'active' | 'idle' | 'syncing' | 'offline';
  tasks: number;
  hashrate: number;
}

interface LeafletMapProps {
  nodes: GpuNode[];
  selectedNode: GpuNode | null;
  onNodeSelect: (node: GpuNode | null) => void;
}

// Custom cyberpunk marker icon creator
const createCyberpunkMarker = (status: string, isSelected: boolean = false) => {
  const colors: Record<string, { main: string; glow: string }> = {
    active: { main: '#00ff9d', glow: 'rgba(0, 255, 157, 0.8)' },
    idle: { main: '#fbbf24', glow: 'rgba(251, 191, 36, 0.6)' },
    syncing: { main: '#06b6d4', glow: 'rgba(6, 182, 212, 0.7)' },
    offline: { main: '#ef4444', glow: 'rgba(239, 68, 68, 0.6)' },
  };
  
  const color = colors[status] || colors.offline;
  const size = isSelected ? 24 : 16;
  const pulseSize = isSelected ? 40 : 28;
  
  const svgIcon = `
    <svg width="${pulseSize}" height="${pulseSize}" viewBox="0 0 ${pulseSize} ${pulseSize}" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <filter id="glow-${status}" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
          <feMerge>
            <feMergeNode in="coloredBlur"/>
            <feMergeNode in="SourceGraphic"/>
          </feMerge>
        </filter>
        <radialGradient id="gradient-${status}" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stop-color="${color.main}" stop-opacity="1"/>
          <stop offset="100%" stop-color="${color.main}" stop-opacity="0.3"/>
        </radialGradient>
      </defs>
      ${status === 'active' ? `
        <circle cx="${pulseSize/2}" cy="${pulseSize/2}" r="${pulseSize/2 - 2}" fill="none" stroke="${color.glow}" stroke-width="1" opacity="0.3">
          <animate attributeName="r" from="${size/2}" to="${pulseSize/2}" dur="2s" repeatCount="indefinite"/>
          <animate attributeName="opacity" from="0.6" to="0" dur="2s" repeatCount="indefinite"/>
        </circle>
      ` : ''}
      <circle cx="${pulseSize/2}" cy="${pulseSize/2}" r="${size/2 + 4}" fill="none" stroke="${color.glow}" stroke-width="2" filter="url(#glow-${status})" opacity="0.6"/>
      <circle cx="${pulseSize/2}" cy="${pulseSize/2}" r="${size/2}" fill="url(#gradient-${status})" filter="url(#glow-${status})"/>
      <circle cx="${pulseSize/2}" cy="${pulseSize/2}" r="${size/4}" fill="${color.main}" filter="url(#glow-${status})"/>
    </svg>
  `;
  
  return L.divIcon({
    html: svgIcon,
    className: 'cyberpunk-marker',
    iconSize: [pulseSize, pulseSize],
    iconAnchor: [pulseSize/2, pulseSize/2],
    popupAnchor: [0, -pulseSize/2],
  });
};

// Custom cyberpunk styled popup
const createPopupContent = (node: GpuNode) => {
  const statusColors: Record<string, string> = {
    active: '#00ff9d',
    idle: '#fbbf24',
    syncing: '#06b6d4',
    offline: '#ef4444',
  };
  
  return `
    <div class="cyberpunk-popup" style="
      background: rgba(0, 0, 0, 0.95);
      border: 1px solid rgba(0, 255, 255, 0.5);
      border-radius: 8px;
      padding: 16px;
      min-width: 220px;
      font-family: 'JetBrains Mono', 'Fira Code', monospace;
      box-shadow: 0 0 20px rgba(0, 255, 255, 0.3), inset 0 0 20px rgba(0, 255, 255, 0.05);
    ">
      <div style="color: #06b6d4; font-size: 10px; text-transform: uppercase; letter-spacing: 2px; margin-bottom: 4px;">
        GPU Node
      </div>
      <div style="color: white; font-size: 16px; font-weight: bold; margin-bottom: 8px;">
        ${node.city}, ${node.country}
      </div>
      <div style="color: #6b7280; font-size: 11px; margin-bottom: 12px;">
        ${node.id}
      </div>
      <div style="display: grid; gap: 8px;">
        <div style="display: flex; justify-content: space-between; align-items: center;">
          <span style="color: #9ca3af; font-size: 11px;">STATUS</span>
          <span style="display: flex; align-items: center; gap: 6px;">
            <span style="
              width: 8px; height: 8px; border-radius: 50%;
              background: ${statusColors[node.status]};
              box-shadow: 0 0 10px ${statusColors[node.status]};
            "></span>
            <span style="color: ${statusColors[node.status]}; text-transform: capitalize; font-size: 12px;">
              ${node.status}
            </span>
          </span>
        </div>
        <div style="display: flex; justify-content: space-between;">
          <span style="color: #9ca3af; font-size: 11px;">GPU</span>
          <span style="color: white; font-size: 12px;">${node.gpu}</span>
        </div>
        <div style="display: flex; justify-content: space-between;">
          <span style="color: #9ca3af; font-size: 11px;">VRAM</span>
          <span style="color: #06b6d4; font-size: 12px;">${node.vram} GB</span>
        </div>
        <div style="display: flex; justify-content: space-between;">
          <span style="color: #9ca3af; font-size: 11px;">TASKS</span>
          <span style="color: #00ff9d; font-size: 12px;">${node.tasks}</span>
        </div>
        <div style="display: flex; justify-content: space-between;">
          <span style="color: #9ca3af; font-size: 11px;">HASHRATE</span>
          <span style="color: #ec4899; font-size: 12px;">${node.hashrate.toFixed(1)} TH/s</span>
        </div>
      </div>
      <div style="
        margin-top: 12px;
        padding-top: 12px;
        border-top: 1px solid rgba(0, 255, 255, 0.2);
      ">
        <div style="display: flex; justify-content: space-between; font-size: 10px;">
          <span style="color: #6b7280;">COORDINATES</span>
          <span style="color: #9ca3af;">${node.lat.toFixed(4)}°, ${node.lng.toFixed(4)}°</span>
        </div>
      </div>
    </div>
  `;
};

export default function LeafletMap({ nodes, selectedNode, onNodeSelect }: LeafletMapProps) {
  const mapRef = useRef<HTMLDivElement>(null);
  const mapInstanceRef = useRef<L.Map | null>(null);
  const markersRef = useRef<Map<string, L.Marker>>(new Map());
  const polylineRef = useRef<L.Polyline | null>(null);
  
  // Initialize map
  useEffect(() => {
    if (!mapRef.current || mapInstanceRef.current) return;
    
    // Create map with dark cyberpunk theme
    const map = L.map(mapRef.current, {
      center: [20, 0],
      zoom: 2,
      minZoom: 2,
      maxZoom: 10,
      zoomControl: false,
      attributionControl: false,
    });
    
    // Add custom zoom control position
    L.control.zoom({ position: 'bottomright' }).addTo(map);
    
    // Use CartoDB Dark Matter tiles for cyberpunk look
    L.tileLayer('https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png', {
      maxZoom: 19,
    }).addTo(map);
    
    // Add custom CSS for cyberpunk effects
    const style = document.createElement('style');
    style.textContent = `
      .leaflet-container {
        background: #0a0a0f !important;
        font-family: 'JetBrains Mono', monospace;
      }
      .leaflet-tile {
        filter: saturate(0.3) brightness(0.8) hue-rotate(180deg) !important;
      }
      .leaflet-popup-content-wrapper {
        background: transparent !important;
        box-shadow: none !important;
        padding: 0 !important;
      }
      .leaflet-popup-content {
        margin: 0 !important;
      }
      .leaflet-popup-tip-container {
        display: none !important;
      }
      .leaflet-popup-close-button {
        color: #06b6d4 !important;
        font-size: 20px !important;
        padding: 8px !important;
      }
      .leaflet-popup-close-button:hover {
        color: #00ff9d !important;
      }
      .cyberpunk-marker {
        background: transparent !important;
        border: none !important;
      }
      .leaflet-control-zoom {
        border: 1px solid rgba(0, 255, 255, 0.3) !important;
        background: rgba(0, 0, 0, 0.8) !important;
      }
      .leaflet-control-zoom a {
        background: rgba(0, 0, 0, 0.9) !important;
        color: #06b6d4 !important;
        border-color: rgba(0, 255, 255, 0.2) !important;
      }
      .leaflet-control-zoom a:hover {
        background: rgba(0, 255, 255, 0.1) !important;
        color: #00ff9d !important;
      }
    `;
    document.head.appendChild(style);
    
    mapInstanceRef.current = map;
    
    return () => {
      map.remove();
      mapInstanceRef.current = null;
      style.remove();
    };
  }, []);
  
  // Update markers when nodes change
  useEffect(() => {
    const map = mapInstanceRef.current;
    if (!map) return;
    
    // Clear old markers that are no longer in nodes
    markersRef.current.forEach((marker, id) => {
      if (!nodes.find(n => n.id === id)) {
        map.removeLayer(marker);
        markersRef.current.delete(id);
      }
    });
    
    // Add/update markers
    nodes.forEach(node => {
      const existingMarker = markersRef.current.get(node.id);
      const isSelected = selectedNode?.id === node.id;
      
      if (existingMarker) {
        // Update marker position and icon
        existingMarker.setLatLng([node.lat, node.lng]);
        existingMarker.setIcon(createCyberpunkMarker(node.status, isSelected));
        existingMarker.setPopupContent(createPopupContent(node));
      } else {
        // Create new marker
        const marker = L.marker([node.lat, node.lng], {
          icon: createCyberpunkMarker(node.status, isSelected),
        });
        
        marker.bindPopup(createPopupContent(node), {
          closeButton: true,
          className: 'cyberpunk-popup-wrapper',
        });
        
        marker.on('click', () => {
          onNodeSelect(selectedNode?.id === node.id ? null : node);
        });
        
        marker.addTo(map);
        markersRef.current.set(node.id, marker);
      }
    });
    
    // Draw connection lines between active nodes
    if (polylineRef.current) {
      map.removeLayer(polylineRef.current);
    }
    
    const activeNodes = nodes.filter(n => n.status === 'active');
    if (activeNodes.length > 1) {
      const connectionPairs: [L.LatLngExpression, L.LatLngExpression][] = [];
      
      // Create a network of connections (not just linear)
      for (let i = 0; i < activeNodes.length; i++) {
        for (let j = i + 1; j < Math.min(i + 3, activeNodes.length); j++) {
          connectionPairs.push([
            [activeNodes[i].lat, activeNodes[i].lng],
            [activeNodes[j].lat, activeNodes[j].lng],
          ]);
        }
      }
      
      // Draw all connections as a single polyline group
      connectionPairs.forEach(([from, to]) => {
        const line = L.polyline([from, to], {
          color: 'rgba(0, 255, 255, 0.15)',
          weight: 1,
          apps/dash-legacy-2-legacy-2Array: '5, 10',
          className: 'connection-line',
        }).addTo(map);
      });
    }
    
  }, [nodes, selectedNode, onNodeSelect]);
  
  // Pan to selected node
  useEffect(() => {
    const map = mapInstanceRef.current;
    if (!map || !selectedNode) return;
    
    map.setView([selectedNode.lat, selectedNode.lng], 5, {
      animate: true,
      duration: 1,
    });
    
    // Open popup for selected node
    const marker = markersRef.current.get(selectedNode.id);
    if (marker) {
      marker.openPopup();
    }
  }, [selectedNode]);
  
  return (
    <div 
      ref={mapRef} 
      className="w-full h-[600px]"
      style={{
        background: 'linear-gradient(180deg, #0a0a0f 0%, #0f172a 100%)',
      }}
    />
  );
}
