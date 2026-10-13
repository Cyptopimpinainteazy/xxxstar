'use client';

import React from 'react';
import Link from 'next/link';

interface LogoProps {
  size?: 'sm' | 'md' | 'lg';
  showText?: boolean;
  variant?: 'default' | 'light' | 'dark';
}

export default function Logo({ size = 'md', showText = true, variant = 'default' }: LogoProps) {
  const sizes = {
    sm: { container: 'w-8 h-8', text: 'text-base', subtext: 'text-[10px]' },
    md: { container: 'w-10 h-10', text: 'text-lg', subtext: 'text-xs' },
    lg: { container: 'w-14 h-14', text: 'text-2xl', subtext: 'text-sm' },
  };

  return (
    <Link href="/" className="flex items-center space-x-3 group">
      {/* Hexagon Logo */}
      <div className={`relative ${sizes[size].container}`}>
        {/* Outer glow */}
        <div className="absolute inset-0 bg-gradient-to-br from-orange-500 via-amber-500 to-orange-600 rounded-xl opacity-80 blur-sm group-hover:opacity-100 transition-opacity" />
        
        {/* Main container */}
        <div className="absolute inset-0 bg-gradient-to-br from-orange-500 via-amber-500 to-orange-600 rounded-xl" />
        
        {/* Inner dark background */}
        <div className="absolute inset-0.5 bg-slate-950 rounded-[10px] flex items-center justify-center overflow-hidden">
          {/* Hexagon pattern background */}
          <svg 
            viewBox="0 0 100 100" 
            className="absolute inset-0 w-full h-full opacity-30"
          >
            <defs>
              <linearGradient id="hexGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stopColor="#f97316" />
                <stop offset="50%" stopColor="#fbbf24" />
                <stop offset="100%" stopColor="#06b6d4" />
              </linearGradient>
            </defs>
            {/* Center hexagon cluster */}
            <polygon 
              points="50,20 65,30 65,50 50,60 35,50 35,30" 
              fill="none" 
              stroke="url(#hexGradient)" 
              strokeWidth="2"
            />
            <polygon 
              points="65,50 80,60 80,80 65,90 50,80 50,60" 
              fill="none" 
              stroke="url(#hexGradient)" 
              strokeWidth="1.5"
              opacity="0.7"
            />
            <polygon 
              points="35,50 50,60 50,80 35,90 20,80 20,60" 
              fill="none" 
              stroke="url(#hexGradient)" 
              strokeWidth="1.5"
              opacity="0.7"
            />
          </svg>
          
          {/* X3 Text */}
          <span className="relative z-10 text-xl font-bold bg-gradient-to-r from-orange-400 via-amber-400 to-cyan-400 bg-clip-text text-transparent">
            X3
          </span>
        </div>
      </div>

      {/* Text */}
      {showText && (
        <div className="flex flex-col">
          <span className={`${sizes[size].text} font-bold bg-gradient-to-r from-orange-400 via-amber-400 to-orange-500 bg-clip-text text-transparent`}>
            X3 STAR
          </span>
          <span className={`${sizes[size].subtext} text-gray-400`}>
            Atlas Sphere Network
          </span>
        </div>
      )}
    </Link>
  );
}

// Animated hexagon background component
export function HexagonBackground() {
  return (
    <div className="absolute inset-0 overflow-hidden pointer-events-none">
      <svg 
        className="absolute w-full h-full opacity-10"
        viewBox="0 0 1000 1000"
        preserveAspectRatio="xMidYMid slice"
      >
        <defs>
          <linearGradient id="hexBgGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#f97316" />
            <stop offset="50%" stopColor="#a855f7" />
            <stop offset="100%" stopColor="#06b6d4" />
          </linearGradient>
          <pattern id="hexPattern" width="100" height="86.6" patternUnits="userSpaceOnUse">
            <polygon 
              points="50,0 100,25 100,75 50,100 0,75 0,25" 
              fill="none" 
              stroke="url(#hexBgGradient)" 
              strokeWidth="1"
              transform="translate(0, -13.3)"
            />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#hexPattern)" />
      </svg>
      
      {/* Animated floating hexagons */}
      <div className="absolute top-1/4 left-1/4 w-32 h-32 animate-float">
        <svg viewBox="0 0 100 100" className="w-full h-full">
          <polygon 
            points="50,5 95,27.5 95,72.5 50,95 5,72.5 5,27.5" 
            fill="none" 
            stroke="rgba(249, 115, 22, 0.3)" 
            strokeWidth="2"
          />
        </svg>
      </div>
      
      <div className="absolute top-2/3 right-1/4 w-24 h-24 animate-float" style={{ animationDelay: '2s' }}>
        <svg viewBox="0 0 100 100" className="w-full h-full">
          <polygon 
            points="50,5 95,27.5 95,72.5 50,95 5,72.5 5,27.5" 
            fill="none" 
            stroke="rgba(6, 182, 212, 0.3)" 
            strokeWidth="2"
          />
        </svg>
      </div>
      
      <div className="absolute top-1/2 right-1/6 w-20 h-20 animate-float" style={{ animationDelay: '4s' }}>
        <svg viewBox="0 0 100 100" className="w-full h-full">
          <polygon 
            points="50,5 95,27.5 95,72.5 50,95 5,72.5 5,27.5" 
            fill="none" 
            stroke="rgba(168, 85, 247, 0.3)" 
            strokeWidth="2"
          />
        </svg>
      </div>
    </div>
  );
}

// Glowing hexagon cluster for hero sections
export function HexagonCluster({ className = '' }: { className?: string }) {
  return (
    <div className={`relative ${className}`}>
      <svg viewBox="0 0 200 200" className="w-full h-full">
        <defs>
          <linearGradient id="clusterGradient1" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#f97316" />
            <stop offset="100%" stopColor="#fbbf24" />
          </linearGradient>
          <linearGradient id="clusterGradient2" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#a855f7" />
            <stop offset="100%" stopColor="#6366f1" />
          </linearGradient>
          <linearGradient id="clusterGradient3" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#06b6d4" />
            <stop offset="100%" stopColor="#22d3ee" />
          </linearGradient>
          <filter id="glow">
            <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
            <feMerge>
              <feMergeNode in="coloredBlur"/>
              <feMergeNode in="SourceGraphic"/>
            </feMerge>
          </filter>
        </defs>
        
        {/* Center hexagon - glowing */}
        <polygon 
          points="100,50 135,70 135,110 100,130 65,110 65,70" 
          fill="url(#clusterGradient1)"
          fillOpacity="0.3"
          stroke="url(#clusterGradient1)" 
          strokeWidth="2"
          filter="url(#glow)"
          className="animate-pulse-slow"
        />
        
        {/* Top left */}
        <polygon 
          points="65,30 100,50 100,90 65,110 30,90 30,50" 
          fill="none" 
          stroke="url(#clusterGradient2)" 
          strokeWidth="1.5"
          opacity="0.7"
        />
        
        {/* Top right */}
        <polygon 
          points="135,30 170,50 170,90 135,110 100,90 100,50" 
          fill="none" 
          stroke="url(#clusterGradient2)" 
          strokeWidth="1.5"
          opacity="0.7"
        />
        
        {/* Bottom */}
        <polygon 
          points="100,90 135,110 135,150 100,170 65,150 65,110" 
          fill="none" 
          stroke="url(#clusterGradient3)" 
          strokeWidth="1.5"
          opacity="0.7"
        />
        
        {/* Outer hexagons */}
        <polygon 
          points="30,70 65,90 65,130 30,150 -5,130 -5,90" 
          fill="none" 
          stroke="url(#clusterGradient3)" 
          strokeWidth="1"
          opacity="0.4"
        />
        <polygon 
          points="170,70 205,90 205,130 170,150 135,130 135,90" 
          fill="none" 
          stroke="url(#clusterGradient3)" 
          strokeWidth="1"
          opacity="0.4"
        />
      </svg>
    </div>
  );
}
