import { z } from "zod";

/**
 * Mirrors `ErrorCode` in `src-tauri/src/error.rs`.
 *
 * Kept as a Zod enum so an unrecognised code from a newer backend fails validation loudly
 * instead of silently rendering as an empty state.
 */
export const errorCodeSchema = z.enum([
  "io",
  "config",
  "invalidPath",
  "processNotFound",
  "processFailed",
  "gitNotFound",
  "gitFailed",
  "notARepository",
  "directoryNotEmpty",
  "dirtyWorktree",
  "detachedHead",
  "noUpstream",
  "notFastForward",
  "unknownRef",
  "remoteExists",
  "remoteNotFound",
  "sourceNotFound",
  "sourceExists",
  "toolchainIncomplete",
  "configureFailed",
  "buildFailed",
  "buildCancelled",
  "buildArtifactMissing",
  "buildInProgress",
  "runtimeExists",
  "runtimeNotFound",
  "capabilityDiscoveryFailed",
  "invalidGguf",
  "modelScanFailed",
  "modelDownloadFailed",
  "modelDownloadInProgress",
  "modelDownloadCancelled",
  "invalidProfile",
  "profileNotFound",
  "portInUse",
  "serverAlreadyRunning",
  "serverNotRunning",
  "serverStartFailed",
  "benchmarkFailed",
  "benchmarkInProgress",
  "benchmarkCancelled",
  "unsupported",
  "internal",
]);

export type ErrorCode = z.infer<typeof errorCodeSchema>;

export const appErrorSchema = z.object({
  code: errorCodeSchema,
  message: z.string(),
  hint: z.string().nullish(),
  details: z.string().nullish(),
});

export type AppError = z.infer<typeof appErrorSchema>;

export function isAppError(value: unknown): value is AppError {
  return appErrorSchema.safeParse(value).success;
}

/** Normalizes anything thrown across the IPC boundary into a displayable error. */
export function toAppError(value: unknown): AppError {
  const parsed = appErrorSchema.safeParse(value);
  if (parsed.success) {
    return parsed.data;
  }

  if (value instanceof Error) {
    return { code: "internal", message: value.message };
  }

  return { code: "internal", message: String(value) };
}
