/**
 * Three.js configuration — color palette & rendering settings.
 * Shared by SceneManager and any UI components that need the accent colors.
 */
export const Config = {
  colors: {
    bg:      0x050508,
    primary: 0xff6b35,   // orange
    secondary: 0x00b4ff, // electric blue
    accent:  0xff8c42,   // amber
    purple:  0x8b5cf6,
    grid:    0xff6b35,
  },
  rendering: {
    pixelRatio: Math.min(window.devicePixelRatio, 2),
    bloomStrength: 1.6,
    bloomThreshold: 0.28,
    bloomRadius: 0.7,
  },
};
