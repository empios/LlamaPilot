import { create } from "zustand";

interface UpdateUiState {
  working: boolean;
  installing: boolean;
  countdown: number | null;
  error: string | null;
  blockers: Record<string, boolean>;
}

export const useUpdateStore = create<UpdateUiState>(() => ({
  working: false, installing: false, countdown: null, error: null, blockers: {},
}));

export function hasUpdateBlockers() {
  return Object.values(useUpdateStore.getState().blockers).some(Boolean)
    || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null;
}
