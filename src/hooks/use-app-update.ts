import { useCallback } from "react";
import { flushSync } from "react-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { isTauri } from "@tauri-apps/api/core";
import { ipc } from "@/lib/ipc";
import { toAppError } from "@/types/errors";
import { hasUpdateBlockers, useUpdateStore } from "@/stores/update-store";

export const updateQueryKey = ["application-update"] as const;

export function useAppUpdate() {
  return useQuery({
    queryKey: updateQueryKey, queryFn: ipc.getAppUpdateStatus,
    enabled: isTauri(), refetchInterval: 1000, retry: false,
  });
}

export type UpdateAction = "check" | "download" | "install" | "defer";

export function useUpdateActions() {
  const client = useQueryClient();
  return useCallback(async (action: UpdateAction, automatic = false) => {
    if (useUpdateStore.getState().working) return;
    if (action === "install" && hasUpdateBlockers()) {
      useUpdateStore.setState({ countdown: null, error: "Save or close open editors before restarting." });
      return;
    }
    // Commit the blocking overlay before sending IPC; no new UI edits can slip into restart.
    flushSync(() => useUpdateStore.setState({ working: true, installing: action === "install", error: null, countdown: null }));
    try {
      if (action === "check") await ipc.checkAppUpdate();
      if (action === "download") await ipc.downloadAppUpdate();
      if (action === "install") await ipc.installAppUpdate(automatic);
      if (action === "defer") await ipc.deferAppUpdate();
    } catch (error) {
      const failure = toAppError(error);
      // Busy is a deferral, not a failed installer. The scheduler starts a fresh
      // countdown after work finishes, including work that raced the status poll.
      useUpdateStore.setState({ error: automatic && failure.code === "updateBusy" ? null : failure.message });
    } finally {
      useUpdateStore.setState({ working: false, installing: false });
      await client.invalidateQueries({ queryKey: updateQueryKey });
    }
  }, [client]);
}
