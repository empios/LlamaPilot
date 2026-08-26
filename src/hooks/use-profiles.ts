import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { showAppError } from "@/lib/toast-error";
import type { ProfileInput } from "@/types/profiles";

export function useProfiles() {
  return useQuery({
    queryKey: queryKeys.profiles,
    queryFn: ipc.listProfiles,
  });
}

export function useProfilePreview(
  profileId: string | null,
  input: ProfileInput,
  enabled: boolean,
) {
  return useQuery({
    queryKey: queryKeys.profilePreview(profileId, input),
    queryFn: () => ipc.previewProfileCommand(profileId, input),
    enabled,
    retry: false,
    staleTime: 10_000,
  });
}

export function useCreateProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ipc.createProfile,
    onSuccess: (profile) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.profiles });
      toast.success(`Profile “${profile.name}” created`);
    },
    onError: showAppError,
  });
}

export function useUpdateProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: ProfileInput }) =>
      ipc.updateProfile(id, input),
    onSuccess: (profile) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.profiles });
      toast.success(`Profile “${profile.name}” saved`);
    },
    onError: showAppError,
  });
}

export function useDeleteProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ipc.deleteProfile,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.profiles });
      toast.success("Profile deleted");
    },
    onError: showAppError,
  });
}
