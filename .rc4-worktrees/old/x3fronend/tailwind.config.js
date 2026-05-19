/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './index.html',
    './src/**/*.{js,jsx,ts,tsx}'
  ],
  theme: {
    extend: {
      colors: {
        ink: '#000008',
        cream: '#F0F4FF',
        'surface-dark': 'rgba(0,0,8,0.3)',
        gold: '#FFD700',
        cyan: '#00E5FF',
        green: '#00FF88'
      },
      fontFamily: {
        display: ['"Druk Wide"', 'sans-serif'],
        body: ['"GT America"', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace']
      },
      animation: {
        'spin-slow': 'spin 40s linear infinite',
        ripple: 'ripple 1s ease-in-out infinite'
      },
      keyframes: {
        ripple: {
          '0%,100%': { transform: 'scale(1)' },
          '50%': { transform: 'scale(1.1)' }
        }
      }
    }
  },
  plugins: []
};