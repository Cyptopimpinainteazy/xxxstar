import React from 'react';
import clsx from 'clsx';

export default function Button({ children, primary, outline, className = '', ...props }) {
  return (
    <button
      className={clsx(
        'relative overflow-hidden font-display tracking-widest transition transform focus:outline-none focus:ring',
        primary && 'bg-gradient-to-tr from-gold to-cyan text-black shadow-lg',
        outline && 'border border-cream text-cream hover:bg-cream/10',
        'px-6 py-3 rounded-lg',
        'active:scale-95',
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
}
