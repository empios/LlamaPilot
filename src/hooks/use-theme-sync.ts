import { useEffect } from "react";

import { useSettings } from "@/hooks/use-settings";
import { resolveTheme, useThemeStore } from "@/stores/theme-store";

/**
 * Keeps the document class in sync with the persisted theme preference, and follows the OS
 * setting while the preference is "system".
 */
export function useThemeSync() {
  const preference = useThemeStore((state) => state.preference);
  const setPreference = useThemeStore((state) => state.setPreference);
  const settings = useSettings();

  const persistedTheme = settings.data?.appearance.theme;

  useEffect(() => {
    if (persistedTheme) {
      setPreference(persistedTheme);
    }
  }, [persistedTheme, setPreference]);

  useEffect(() => {
    const apply = () => {
      const resolved = resolveTheme(preference);
      document.documentElement.classList.toggle("dark", resolved === "dark");
      document.documentElement.dataset.theme =
        resolved === "dark" ? "terminal" : "paper";
      document.documentElement.style.colorScheme = resolved;
    };

    apply();

    if (preference !== "system") {
      return;
    }

    const media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [preference]);
}
