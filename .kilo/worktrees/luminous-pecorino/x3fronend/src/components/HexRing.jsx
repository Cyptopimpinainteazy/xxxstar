import React from 'react';

// simple hexagon ring SVG with customizable className
export default function HexRing({ className = '' }) {
  return (
    <svg
      className={className}
      viewBox="0 0 200 200"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <polygon
        points="100,10 180,55 180,145 100,190 20,145 20,55"
        stroke="url(#grad)"
        strokeWidth="4"
      />
      <defs>
        <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#FFD700" />
          <stop offset="100%" stopColor="#00E5FF" />
        </linearGradient>
      </defs>
    </svg>
  );
}
