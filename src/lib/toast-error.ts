import { toast } from "sonner";

import { toAppError } from "@/types/errors";

/** Shows every structured backend error consistently, including raw details when no hint exists. */
export function showAppError(error: unknown): void {
  const appError = toAppError(error);
  toast.error(appError.message, {
    description: appError.hint ?? appError.details ?? undefined,
  });
}
