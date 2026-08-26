import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { Channel } from "@tauri-apps/api/core";
import { toast } from "sonner";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import { toAppError } from "@/types/errors";
import { backendLabel, type BuildRequest } from "@/types/build";
import type { ProgressEvent } from "@/types/sources";

export function useToolchain() {
  return useQuery({
    queryKey: queryKeys.toolchain,
    queryFn: ipc.detectToolchain,
    // Detection shells out to vswhere, cmake, and nvcc, so it is not cheap to repeat.
    staleTime: 5 * 60_000,
  });
}

export function useRuntimes() {
  return useQuery({
    queryKey: queryKeys.runtimes,
    queryFn: ipc.listRuntimes,
  });
}

export function useRuntimeCapabilities(id: string | null, enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.runtimeCapabilities(id ?? "none"),
    queryFn: () => ipc.getRuntimeCapabilities(id as string),
    enabled: enabled && id !== null,
  });
}

export function useInspectRuntimeCapabilities() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => ipc.inspectRuntimeCapabilities(id),
    onSuccess: (inspection, id) => {
      queryClient.setQueryData(queryKeys.runtimeCapabilities(id), inspection);
      void queryClient.invalidateQueries({ queryKey: queryKeys.runtimes });
      toast.success("Runtime inspected", {
        description: `${Object.keys(inspection.capabilities.options).length} options and ${inspection.capabilities.devices.length} devices detected.`,
      });
    },
    onError: showAppError,
  });
}

export function useBuildStatus() {
  return useQuery({
    queryKey: queryKeys.buildStatus,
    queryFn: ipc.isBuildRunning,
    staleTime: 0,
    refetchOnMount: "always",
    refetchInterval: (query) => (query.state.data ? 1_000 : false),
  });
}

export function useBuildRuntime() {
  const queryClient = useQueryClient();

  return useMutation({
    onMutate: () => {
      queryClient.setQueryData(queryKeys.buildStatus, true);
    },
    mutationFn: ({
      request,
      channel,
    }: {
      request: BuildRequest;
      channel: Channel<ProgressEvent>;
    }) => ipc.buildRuntime(request, channel),
    onSuccess: (outcome) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.runtimes });
      const { branch, shortCommit, backend, fileCount } = outcome.runtime;
      toast.success(`Built ${branch} @ ${shortCommit} · ${backendLabel(backend)}`, {
        description: `${fileCount} files copied into the runtime.`,
      });
    },
    onError: (error) => {
      const appError = toAppError(error);
      // A cancellation is a user action, not a failure to apologise for.
      if (appError.code === "buildCancelled") {
        toast.info("Build cancelled");
        return;
      }
      toast.error(appError.message, { description: appError.hint ?? undefined });
    },
    onSettled: () => {
      queryClient.setQueryData(queryKeys.buildStatus, false);
    },
  });
}

export function useCancelBuild() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ipc.cancelBuild,
    onSuccess: (wasRunning) => {
      if (!wasRunning) {
        toast.info("No build was running");
      }
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.buildStatus });
    },
  });
}

export function useDeleteRuntime() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => ipc.deleteRuntime(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.runtimes });
      toast.success("Runtime deleted");
    },
    onError: showAppError,
  });
}
