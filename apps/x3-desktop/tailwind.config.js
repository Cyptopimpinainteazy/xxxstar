/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        bg: {
          primary: '#07070d',
          card: '#0d0d1a',
          surface: '#1a1a2e',
        },
        text: {
          primary: '#e0e0e0',
          muted: '#666',
        },
        accent: {
          DEFAULT: '#ff6b35',
          secondary: '#00e5c3',
        },
        neon: {
          cyan: '#00ddff',
          blue: '#4488ff',
          green: '#00cc66',
          orange: '#ff8800',
          pink: '#ff3366',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'monospace'],
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
      },
      backdropBlur: {
        xs: '2px',
      },
    },
  },
  plugins: [],
};
