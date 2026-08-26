import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";

export function useAppInfo() {
  return useQuery({
    queryKey: queryKeys.appInfo,
    queryFn: ipc.getAppInfo,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useHardwareSnapshot() {
  return useQuery({
    queryKey: queryKeys.hardware,
    queryFn: ipc.getHardwareSnapshot,
    refetchInterval: 30_000,
    // VRAM figures move constantly; swapping to a skeleton on every poll would strobe.
    placeholderData: keepPreviousData,
  });
}

export function useGitVersion() {
  return useQuery({
    queryKey: queryKeys.gitVersion,
    queryFn: ipc.getGitVersion,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
}
