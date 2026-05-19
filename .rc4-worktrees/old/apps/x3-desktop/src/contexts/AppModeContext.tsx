/**
 * AppModeContext.tsx — Controls normal vs exclusive mode
 * 
 * Normal Mode: Just the eyeball (standard users)
 * Exclusive Mode: Eyeball + Pyramid + Metatron cube (exclusive rights holders)
 * 
 * Mode is determined by:
 *  - URL param: ?mode=exclusive
 *  - localStorage: x3-desktop:mode
 *  - Default: normal
 */
import { createContext, useContext, useState, useEffect, type ReactNode } from 'react';

export type AppMode = 'normal' | 'exclusive';

interface AppModeContextType {
  mode: AppMode;
  setMode: (mode: AppMode) => void;
  isExclusive: boolean;
}

const AppModeContext = createContext<AppModeContextType | null>(null);

const STORAGE_KEY = 'x3-desktop:mode';

function getInitialMode(): AppMode {
  // Check URL param first
  const urlParams = new URLSearchParams(window.location.search);
  const urlMode = urlParams.get('mode');
  if (urlMode === 'exclusive') return 'exclusive';
  if (urlMode === 'normal') return 'normal';
  
  // Check localStorage
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'exclusive') return 'exclusive';
  } catch {
    // Ignore storage errors
  }
  
  return 'normal';
}

export function AppModeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<AppMode>(getInitialMode);

  const setMode = (newMode: AppMode) => {
    setModeState(newMode);
    try {
      localStorage.setItem(STORAGE_KEY, newMode);
    } catch {
      // Ignore storage errors
    }
  };

  // Apply mode class to body for CSS targeting
  useEffect(() => {
    document.body.classList.remove('mode-normal', 'mode-exclusive');
    document.body.classList.add(`mode-${mode}`);
  }, [mode]);

  return (
    <AppModeContext.Provider value={{ mode, setMode, isExclusive: mode === 'exclusive' }}>
      {children}
    </AppModeContext.Provider>
  );
}

export function useAppMode() {
  const context = useContext(AppModeContext);
  if (!context) {
    throw new Error('useAppMode must be used within AppModeProvider');
  }
  return context;
}
