import { create } from "zustand";

import type { ThemePreference } from "@/types/settings";

/** Mirrors the resolved preference so `index.html` can avoid a flash of the wrong theme. */
const STORAGE_KEY = "llamapilot.theme";
const LEGACY_STORAGE_KEY = "llama-control.theme";

function readInitialPreference(): ThemePreference {
  if (typeof localStorage === "undefined") {
    return "system";
  }

  const stored = localStorage.getItem(STORAGE_KEY) ?? localStorage.getItem(LEGACY_STORAGE_KEY);
  return stored === "light" || stored === "dark" || stored === "system" ? stored : "system";
}

interface ThemeState {
  preference: ThemePreference;
  setPreference: (preference: ThemePreference) => void;
}

/**
 * Mirrors the persisted appearance setting so the shell can react instantly, before the
 * settings mutation round-trips through Rust.
 */
export const useThemeStore = create<ThemeState>((set) => ({
  preference: readInitialPreference(),
  setPreference: (preference) => {
    try {
      localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      // A blocked storage API only costs us the startup flash guard.
    }
    set({ preference });
  },
}));

export function resolveTheme(preference: ThemePreference): "light" | "dark" {
  if (preference !== "system") {
    return preference;
  }

  const prefersDark =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-color-scheme: dark)").matches;

  return prefersDark ? "dark" : "light";
}
