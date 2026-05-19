import React from 'react';

export default function ListItem({ icon, title, text }) {
  return (
    <div className="flex items-start gap-4">
      <span className="text-2xl leading-none">{icon}</span>
      <div>
        <p className="font-display font-bold text-base mb-1">{title}</p>
        <p className="text-sm opacity-70 leading-relaxed">{text}</p>
      </div>
    </div>
  );
}
