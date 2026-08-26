import { z } from "zod";

export const serverLifecycleStateSchema = z.enum([
  "stopped",
  "starting",
  "loading",
  "ready",
  "busy",
  "stopping",
  "crashed",
]);
export type ServerLifecycleState = z.infer<typeof serverLifecycleStateSchema>;

export const serverTelemetrySchema = z.object({
  propsAvailable: z.boolean().nullable(),
  slotsAvailable: z.boolean().nullable(),
  metricsAvailable: z.boolean().nullable(),
  totalSlots: z.number().nullable(),
  busySlots: z.number().nullable(),
  requestsProcessing: z.number().nullable(),
  requestsDeferred: z.number().nullable(),
  promptTokensPerSecond: z.number().nullable(),
  predictedTokensPerSecond: z.number().nullable(),
  buildInfo: z.string().nullable(),
});

export const serverSnapshotSchema = z.object({
  generation: z.number(),
  state: serverLifecycleStateSchema,
  pid: z.number().nullable(),
  profileId: z.string().nullable(),
  profileName: z.string().nullable(),
  runtimeId: z.string().nullable(),
  runtimeLabel: z.string().nullable(),
  modelName: z.string().nullable(),
  host: z.string().nullable(),
  port: z.number().nullable(),
  startedAt: z.string().nullable(),
  stoppedAt: z.string().nullable(),
  exitCode: z.number().nullable(),
  healthStatus: z.number().nullable(),
  healthMessage: z.string().nullable(),
  lastError: z.string().nullable(),
  logFile: z.string().nullable(),
  telemetry: serverTelemetrySchema,
});
export type ServerSnapshot = z.infer<typeof serverSnapshotSchema>;

export const serverLogStreamSchema = z.enum(["system", "stdout", "stderr"]);
export const serverLogLevelSchema = z.enum(["trace", "debug", "info", "warn", "error"]);
export const serverLogFactSchema = z.enum([
  "modelLoad",
  "gpuOffload",
  "kvCache",
  "listening",
  "throughput",
]);

export const serverLogEntrySchema = z.object({
  sequence: z.number(),
  timestamp: z.string(),
  stream: serverLogStreamSchema,
  level: serverLogLevelSchema,
  fact: serverLogFactSchema.nullable(),
  text: z.string(),
});
export type ServerLogEntry = z.infer<typeof serverLogEntrySchema>;

export const serverLogsSnapshotSchema = z.object({
  entries: z.array(serverLogEntrySchema),
  droppedEntries: z.number(),
  nextSequence: z.number(),
  currentFile: z.string().nullable(),
});
export type ServerLogsSnapshot = z.infer<typeof serverLogsSnapshotSchema>;

export const serverEventSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("status"), snapshot: serverSnapshotSchema }),
  z.object({ kind: z.literal("log"), entry: serverLogEntrySchema }),
  z.object({ kind: z.literal("logsCleared"), nextSequence: z.number() }),
]);
export type ServerEvent = z.infer<typeof serverEventSchema>;

export function serverIsActive(state: ServerLifecycleState): boolean {
  return ["starting", "loading", "ready", "busy", "stopping"].includes(state);
}

export function serverStateLabel(state: ServerLifecycleState): string {
  return state.charAt(0).toUpperCase() + state.slice(1);
}
