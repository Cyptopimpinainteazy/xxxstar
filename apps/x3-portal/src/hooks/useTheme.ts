/**
 * Theme hook placeholder
 * 
 * Returns a simple theme object for components that need theme info.
 * In the future this could integrate with next-themes or a custom theme system.
 */

export interface Theme {
  mode: 'dark' | 'light';
  colors: {
    primary: string;
    secondary: string;
    accent: string;
    background: string;
    foreground: string;
  };
}

const defaultTheme: Theme = {
  mode: 'dark',
  colors: {
    primary: '#00ffff', // cyan
    secondary: '#ff00ff', // magenta
    accent: '#ffff00', // yellow
    background: '#000000',
    foreground: '#ffffff',
  },
};

export function useTheme(): Theme {
  // For now, always return dark cyberpunk theme
  // In the future, this could read from context/local storage
  return defaultTheme;
}

export default useTheme;
