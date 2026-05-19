import React from 'react';

export default function Eyebrow({ text }) {
  return (
    <div className="flex items-center gap-3 mb-4">
      <span className="text-xs uppercase tracking-widest text-gold">{text}</span>
      <div className="flex-1 h-px bg-gold/50" />
    </div>
  );
}
