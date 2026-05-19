import React from 'react';

export default function ScrollIndicator() {
  return (
    <div className="flex flex-col items-center gap-2 animate-fade-up cursor-pointer">
      <div className="w-px h-12 bg-gradient-to-b from-gold/50 to-transparent animate-pulse"></div>
      <span className="text-xs uppercase opacity-70">Scroll</span>
    </div>
  );
}
