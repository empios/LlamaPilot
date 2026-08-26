import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { createServerEventChannel, hasTauriRuntime, ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import type { ServerLogsSnapshot, ServerSnapshot } from "@/types/server";

const MAX_LIVE_LOGS = 10_000;

export function useServerStatus() {
  return useQuery({
    queryKey: queryKeys.serverStatus,
    queryFn: ipc.getServerStatus,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useServerLogs() {
  return useQuery({
    queryKey: queryKeys.serverLogs,
    queryFn: ipc.getServerLogs,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

/** Keeps server status and logs live across every page with one IPC subscription. */
export function useServerEventBridge() {
  const queryClient = useQueryClient();
  useServerStatus();
  useServerLogs();

  useEffect(() => {
    if (!hasTauriRuntime()) return;

    let disposed = false;
    let subscriptionId: string | null = null;
    const channel = createServerEventChannel((event) => {
      switch (event.kind) {
        case "status":
          queryClient.setQueryData(queryKeys.serverStatus, event.snapshot);
          break;
        case "log":
          queryClient.setQueryData<ServerLogsSnapshot>(queryKeys.serverLogs, (current) => {
            if (!current) return current;
            const entries = [...current.entries, event.entry];
            const overflow = Math.max(0, entries.length - MAX_LIVE_LOGS);
            return {
              ...current,
              entries: overflow > 0 ? entries.slice(overflow) : entries,
              droppedEntries: current.droppedEntries + overflow,
              nextSequence: event.entry.sequence + 1,
            };
          });
          break;
        case "logsCleared":
          queryClient.setQueryData<ServerLogsSnapshot>(queryKeys.serverLogs, (current) => ({
            entries: [],
            droppedEntries: 0,
            nextSequence: event.nextSequence,
            currentFile: current?.currentFile ?? null,
          }));
          break;
      }
    });

    void ipc
      .subscribeServerEvents(channel)
      .then((id) => {
        if (disposed) {
          void ipc.unsubscribeServerEvents(id);
        } else {
          subscriptionId = id;
        }
      })
      .catch(() => {
        // The initial queries surface connection failures; a second toast here would duplicate it.
      });

    return () => {
      disposed = true;
      if (subscriptionId) {
        void ipc.unsubscribeServerEvents(subscriptionId);
      }
    };
  }, [queryClient]);
}

export function useStartServer() {
  return useServerMutation(ipc.startServer, (snapshot) => {
    toast.success(`Starting “${snapshot.profileName ?? "llama-server"}”`);
  });
}

export function useStopServer() {
  return useServerMutation(ipc.stopServer, () => toast.info("Stopping llama-server"));
}

export function useRestartServer() {
  return useServerMutation(ipc.restartServer, () => toast.info("Restarting llama-server"));
}

export function useClearServerLogs() {
  return useMutation({
    mutationFn: ipc.clearServerLogs,
    onError: showAppError,
  });
}

function useServerMutation<Argument>(
  mutationFn: (argument: Argument) => Promise<ServerSnapshot>,
  onSuccess: (snapshot: ServerSnapshot) => void,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn,
    onSuccess: (snapshot) => {
      queryClient.setQueryData(queryKeys.serverStatus, snapshot);
      onSuccess(snapshot);
    },
    onError: showAppError,
  });
}
