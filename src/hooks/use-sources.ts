import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { toast } from "sonner";
import type { Channel } from "@tauri-apps/api/core";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import type { CloneRequest, ProgressEvent } from "@/types/sources";

export function useSources() {
  return useQuery({
    queryKey: queryKeys.sources,
    queryFn: ipc.listSources,
  });
}

/**
 * Keeping the previous result while a new source loads is what stops the detail pane from
 * collapsing to a skeleton — and the page from jumping — every time the selection changes.
 */
export function useSourceStatus(id: string | null) {
  return useQuery({
    queryKey: queryKeys.sourceStatus(id ?? "none"),
    queryFn: () => ipc.getSourceStatus(id as string),
    enabled: Boolean(id),
    retry: false,
    placeholderData: keepPreviousData,
  });
}

export function useSourceRefs(id: string | null, enabled = true) {
  return useQuery({
    queryKey: queryKeys.sourceRefs(id ?? "none"),
    queryFn: () => ipc.listSourceRefs(id as string),
    enabled: Boolean(id) && enabled,
    retry: false,
    placeholderData: keepPreviousData,
  });
}

export function useSourceRemotes(id: string | null) {
  return useQuery({
    queryKey: queryKeys.sourceRemotes(id ?? "none"),
    queryFn: () => ipc.listSourceRemotes(id as string),
    enabled: Boolean(id),
    retry: false,
    placeholderData: keepPreviousData,
  });
}

/** Invalidates everything derived from a single source's Git state. */
function useSourceInvalidator() {
  const queryClient = useQueryClient();

  return (id: string) => {
    void queryClient.invalidateQueries({ queryKey: queryKeys.sources });
    void queryClient.invalidateQueries({ queryKey: queryKeys.sourceStatus(id) });
    void queryClient.invalidateQueries({ queryKey: queryKeys.sourceRefs(id) });
    void queryClient.invalidateQueries({ queryKey: queryKeys.sourceRemotes(id) });
  };
}

export function useCloneSource() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      request,
      channel,
    }: {
      request: CloneRequest;
      channel: Channel<ProgressEvent>;
    }) => ipc.cloneSource(request, channel),
    onSuccess: (source) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.sources });
      toast.success(`Cloned ${source.name}`);
    },
    onError: showAppError,
  });
}

export function useAddExistingSource() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ directory, name }: { directory: string; name?: string }) =>
      ipc.addExistingSource(directory, name),
    onSuccess: (source) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.sources });
      toast.success(`Added ${source.name}`);
    },
    onError: showAppError,
  });
}

export function useRemoveSource() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, deleteDirectory }: { id: string; deleteDirectory: boolean }) =>
      ipc.removeSource(id, deleteDirectory),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.sources });
      toast.success("Source removed");
    },
    onError: showAppError,
  });
}

export function useFetchSource() {
  const invalidate = useSourceInvalidator();

  return useMutation({
    mutationFn: ({
      id,
      remote,
      channel,
    }: {
      id: string;
      remote: string | null;
      channel: Channel<ProgressEvent>;
    }) => ipc.fetchSource(id, remote, channel),
    onSuccess: (_result, variables) => {
      invalidate(variables.id);
      toast.success("Fetched from remote");
    },
    onError: showAppError,
  });
}

export function useUpdateSource() {
  const invalidate = useSourceInvalidator();

  return useMutation({
    mutationFn: (id: string) => ipc.updateSource(id),
    onSuccess: (outcome, id) => {
      invalidate(id);
      toast.success(
        outcome.changed
          ? `Fast-forwarded to ${outcome.currentCommit?.slice(0, 7) ?? "the latest commit"}`
          : "Already up to date",
      );
    },
    onError: showAppError,
  });
}

export function useSwitchSourceRef() {
  const invalidate = useSourceInvalidator();

  return useMutation({
    mutationFn: ({ id, target }: { id: string; target: string }) =>
      ipc.switchSourceRef(id, target),
    onSuccess: (outcome, variables) => {
      invalidate(variables.id);
      toast.success(`Now on ${outcome.source.currentRef}`, {
        description: outcome.detached
          ? "HEAD is detached. Switch to a branch before updating."
          : undefined,
      });
    },
    onError: showAppError,
  });
}

export function useAddSourceRemote() {
  const invalidate = useSourceInvalidator();

  return useMutation({
    mutationFn: ({ id, name, url }: { id: string; name: string; url: string }) =>
      ipc.addSourceRemote(id, name, url),
    onSuccess: (_remotes, variables) => {
      invalidate(variables.id);
      toast.success(`Added remote ${variables.name}`);
    },
    onError: showAppError,
  });
}

export function useRemoveSourceRemote() {
  const invalidate = useSourceInvalidator();

  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      ipc.removeSourceRemote(id, name),
    onSuccess: (_remotes, variables) => {
      invalidate(variables.id);
      toast.success(`Removed remote ${variables.name}`);
    },
    onError: showAppError,
  });
}
