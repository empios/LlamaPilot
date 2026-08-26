import { create } from "zustand";

import {
  appendProgressEvent,
  type ConsoleLine,
} from "@/components/output-console";
import type { ProgressEvent } from "@/types/sources";

interface BuildLogState {
  lines: ConsoleLine[];
  clear: () => void;
  append: (event: ProgressEvent) => void;
}

/** Keeps live CMake output available when the user navigates away from the Build page. */
export const useBuildLogStore = create<BuildLogState>((set) => ({
  lines: [],
  clear: () => set({ lines: [] }),
  append: (event) =>
    set((state) => ({ lines: appendProgressEvent(state.lines, event) })),
}));
