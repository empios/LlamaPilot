import { create } from "zustand";

/**
 * Pages are a closed union rather than free-form routes, so an unhandled page becomes a
 * compile-time error in the shell's exhaustive switch.
 */
export const PAGE_IDS = [
  "dashboard",
  "models",
  "profiles",
  "performance",
  "runtimes",
  "build",
  "logs",
  "settings",
] as const;

export type PageId = (typeof PAGE_IDS)[number];

interface NavigationState {
  page: PageId;
  selectedSourceId: string | null;
  navigate: (page: PageId) => void;
  selectSource: (sourceId: string | null) => void;
}

export const useNavigationStore = create<NavigationState>((set) => ({
  page: "dashboard",
  selectedSourceId: null,
  navigate: (page) => set({ page }),
  selectSource: (selectedSourceId) => set({ selectedSourceId }),
}));
