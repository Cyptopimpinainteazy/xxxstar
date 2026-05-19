/**
 * Theme store — manages dark/light mode and color preferences.
 *
 * Persists preference to localStorage. Dark mode is default.
 */
import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ThemeMode = "dark" | "light";

export interface ThemeState {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
  toggle: () => void;
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      mode: "dark",
      setMode: (mode) => {
        document.documentElement.classList.toggle("dark", mode === "dark");
        set({ mode });
      },
      toggle: () =>
        set((s) => {
          const next = s.mode === "dark" ? "light" : "dark";
          document.documentElement.classList.toggle("dark", next === "dark");
          return { mode: next };
        }),
    }),
    {
      name: "x3-desktop-theme",
      onRehydrateStorage: () => (state) => {
        if (state) {
          document.documentElement.classList.toggle(
            "dark",
            state.mode === "dark",
          );
        }
      },
    },
  ),
);
