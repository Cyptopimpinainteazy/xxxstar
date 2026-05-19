import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        "bg-primary": "#1a1a1a",
        "bg-secondary": "#2d2d2d",
        "bg-tertiary": "#3a3a3a",
        "bg-surface": "#242424",
        "accent-primary": "#ff6b35",
        "accent-secondary": "#ff8c42",
        "accent-light": "#ffa500",
        "text-primary": "#e0e0e0",
        "text-secondary": "#a8a8a8",
        "text-accent": "#ff8c42",
        "border-default": "#444444",
        "border-active": "#ff6b35",
      },
      boxShadow: {
        glow: "0 0 20px rgba(255, 139, 66, 0.5)",
        "glow-lg": "0 0 40px rgba(255, 139, 66, 0.7)",
        "glow-sm": "0 0 10px rgba(255, 139, 66, 0.3)",
        window:
          "0 8px 32px rgba(0, 0, 0, 0.6), 0 2px 8px rgba(0, 0, 0, 0.4)",
      },
      animation: {
        "pulse-orange": "pulseOrange 2s ease-in-out infinite",
        "fade-in": "fadeIn 0.2s ease-out",
        "slide-up": "slideUp 0.3s ease-out",
        "glow-pulse": "glowPulse 2s ease-in-out infinite",
      },
      keyframes: {
        pulseOrange: {
          "0%, 100%": { opacity: "0.5" },
          "50%": { opacity: "1" },
        },
        fadeIn: {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        slideUp: {
          from: { opacity: "0", transform: "translateY(10px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
        glowPulse: {
          "0%, 100%": { boxShadow: "0 0 10px rgba(255, 139, 66, 0.3)" },
          "50%": { boxShadow: "0 0 25px rgba(255, 139, 66, 0.6)" },
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "system-ui",
          "-apple-system",
          "Segoe UI",
          "sans-serif",
        ],
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
    },
  },
  plugins: [],
};

export default config;
