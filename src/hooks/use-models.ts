import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import type {
  ModelDownloadEvent,
  ModelDownloadRequest,
  ProjectorSelection,
} from "@/types/models";
import type { Channel } from "@tauri-apps/api/core";

export function useModels() {
  return useQuery({
    queryKey: queryKeys.models,
    queryFn: ipc.scanModels,
    staleTime: 30_000,
  });
}

export function useSetModelProjector() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      modelId,
      selection,
    }: {
      modelId: string;
      selection: ProjectorSelection;
    }) => ipc.setModelProjector(modelId, selection),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.models });
      toast.success("Projector choice saved");
    },
    onError: showAppError,
  });
}

export function useInspectHuggingFaceRepository() {
  return useMutation({
    mutationFn: ipc.inspectHuggingFaceRepository,
    onError: showAppError,
  });
}

export function useDownloadHuggingFaceModel() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      request,
      channel,
    }: {
      request: ModelDownloadRequest;
      channel: Channel<ModelDownloadEvent>;
    }) => ipc.downloadHuggingFaceModel(request, channel),
    onSuccess: (outcome) => {
      queryClient.setQueryData(queryKeys.models, outcome.catalog);
      toast.success(
        outcome.files.length === 1
          ? "Model downloaded and added to the catalog"
          : `${outcome.files.length} model shards downloaded and added to the catalog`,
      );
    },
    onError: showAppError,
  });
}
