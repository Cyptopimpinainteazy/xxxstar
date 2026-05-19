/**
 * ThemeProvider — applies dark/light class and provides theme context.
 */
import React, { useEffect } from "react";
import { useThemeStore } from "@/stores/themeStore";

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const mode = useThemeStore((s) => s.mode);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", mode === "dark");
  }, [mode]);

  return <>{children}</>;
};

/**
 * useTheme — convenience hook exposing theme state and toggle.
 */
export function useTheme() {
  const mode = useThemeStore((s) => s.mode);
  const toggle = useThemeStore((s) => s.toggle);
  const setMode = useThemeStore((s) => s.setMode);
  return { mode, toggle, setMode, isDark: mode === "dark" };
}
