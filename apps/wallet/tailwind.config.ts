import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        primary: '#1f2937',
        secondary: '#4f46e5',
        accent: '#ec4899',
        success: '#10b981',
        warning: '#f59e0b',
        danger: '#ef4444',
        dark: '#0f172a',
        'x3-black': '#05070d',
        'x3-darker': '#0b1220',
        'x3-dark': '#111a2b',
        'x3-dark-gray': '#1f2c44',
        'x3-orange': '#f97316',
        'x3-green': '#22c55e',
        'x3-red': '#ef4444',
      },
      backgroundImage: {
        'gradient-primary': 'linear-gradient(135deg, #4f46e5 0%, #ec4899 100%)',
        'gradient-dark': 'linear-gradient(135deg, #1f2937 0%, #111827 100%)',
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'bounce-slow': 'bounce 2s infinite',
      },
    },
  },
  plugins: [],
};

export default config;
