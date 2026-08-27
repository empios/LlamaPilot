import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { Channel } from "@tauri-apps/api/core";
import { toast } from "sonner";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import { toAppError } from "@/types/errors";
import type { PerformanceSweepEvent } from "@/types/performance";

export function usePerformancePlan(profileId: string | null) {
  return useQuery({
    queryKey: queryKeys.performancePlan(profileId ?? "none"),
    queryFn: () => ipc.getPerformancePlan(profileId as string),
    enabled: profileId !== null,
    staleTime: 15_000,
    retry: false,
  });
}

export function usePerformanceBenchmarks(profileId: string | null) {
  return useQuery({
    queryKey: queryKeys.performanceBenchmarks(profileId ?? "all"),
    queryFn: () => ipc.listPerformanceBenchmarks(profileId),
  });
}

export function useRunPerformanceBenchmark() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ipc.runPerformanceBenchmark,
    onSuccess: (result) => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.performanceBenchmarks(result.profileId),
      });
      toast.success("Benchmark completed", {
        description: `${result.promptTokensPerSecond.toFixed(1)} prompt tok/s · ${result.predictedTokensPerSecond.toFixed(1)} generation tok/s`,
      });
    },
    onError: showAppError,
  });
}

export function usePerformanceSweepStatus() {
  return useQuery({
    queryKey: queryKeys.performanceSweepStatus,
    queryFn: ipc.isPerformanceSweepRunning,
    staleTime: 0,
    refetchOnMount: "always",
    refetchInterval: (query) => (query.state.data ? 1_000 : false),
  });
}

export function useRunPerformanceSweep() {
  const queryClient = useQueryClient();
  return useMutation({
    onMutate: () => {
      queryClient.setQueryData(queryKeys.performanceSweepStatus, true);
    },
    mutationFn: ({
      profileId,
      channel,
    }: {
      profileId: string;
      channel: Channel<PerformanceSweepEvent>;
    }) => ipc.runPerformanceSweep(profileId, channel),
    onSuccess: (sweep) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.profiles });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.performancePlan(sweep.profileId),
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.performanceBenchmarks(sweep.profileId),
      });
      void queryClient.invalidateQueries({ queryKey: queryKeys.serverStatus });
      const winner = sweep.results.find(
        (result) => result.candidateId === sweep.winnerCandidateId,
      );
      if (winner?.benchmark) {
        toast.success(`Best placement: ${winner.candidateLabel}`, {
          description: `${winner.benchmark.predictedTokensPerSecond.toFixed(1)} generation tok/s · saved to the profile`,
        });
      } else {
        toast.error("No placement completed successfully", {
          description: "The original profile was restored.",
        });
      }
    },
    onError: (error) => {
      const appError = toAppError(error);
      if (appError.code === "benchmarkCancelled") {
        toast.info("Automatic benchmark cancelled", {
          description: "The original profile was restored.",
        });
      } else {
        showAppError(error);
      }
    },
    onSettled: (_data, _error, variables) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.performanceSweepStatus });
      void queryClient.invalidateQueries({ queryKey: queryKeys.profiles });
      void queryClient.invalidateQueries({ queryKey: queryKeys.serverStatus });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.performancePlan(variables.profileId),
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.performanceBenchmarks(variables.profileId),
      });
    },
  });
}

export function useCancelPerformanceSweep() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ipc.cancelPerformanceSweep,
    onSuccess: (wasRunning) => {
      if (!wasRunning) toast.info("No automatic benchmark is running");
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.performanceSweepStatus });
    },
    onError: showAppError,
  });
}
