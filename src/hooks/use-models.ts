import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import type { ProjectorSelection } from "@/types/models";

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
