import { useMutation } from "@tanstack/react-query";

import { ipc } from "@/lib/ipc";
import { showAppError } from "@/lib/toast-error";

export function useTestAgentConnection() {
  return useMutation({
    mutationFn: ipc.testAgentConnection,
    onError: showAppError,
  });
}
