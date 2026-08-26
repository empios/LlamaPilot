import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import type { Settings } from "@/types/settings";

export function useSettings() {
  return useQuery({
    queryKey: queryKeys.settings,
    queryFn: ipc.getSettings,
    staleTime: 30_000,
  });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (settings: Settings) => ipc.updateSettings(settings),
    onSuccess: (settings) => {
      queryClient.setQueryData(queryKeys.settings, settings);
      void queryClient.invalidateQueries({ queryKey: queryKeys.models });
      toast.success("Settings saved");
    },
    onError: showAppError,
  });
}

export function useResetSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ipc.resetSettings,
    onSuccess: (settings) => {
      queryClient.setQueryData(queryKeys.settings, settings);
      void queryClient.invalidateQueries({ queryKey: queryKeys.models });
      toast.success("Settings restored to defaults");
    },
    onError: showAppError,
  });
}
