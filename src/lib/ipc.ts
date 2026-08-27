import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import type { z } from "zod";

import {
  agentConnectionTestSchema,
  type AgentConnectionTest,
  type AgentConnectionTestRequest,
} from "@/types/agent";
import { toAppError } from "@/types/errors";
import {
  appInfoSchema,
  settingsSchema,
  type AppInfo,
  type Settings,
} from "@/types/settings";
import { hardwareSnapshotSchema, type HardwareSnapshot } from "@/types/hardware";
import {
  gitRemoteSchema,
  llamaSourceSchema,
  sourceRefsSchema,
  sourceStatusSchema,
  switchRefOutcomeSchema,
  updateOutcomeSchema,
  type CloneRequest,
  type GitRemote,
  type LlamaSource,
  type ProgressEvent,
  type SourceRefs,
  type SourceStatus,
  type SwitchRefOutcome,
  type UpdateOutcome,
} from "@/types/sources";
import {
  buildOutcomeSchema,
  runtimeRecordSchema,
  toolchainSchema,
  type BuildOutcome,
  type BuildRequest,
  type RuntimeRecord,
  type Toolchain,
} from "@/types/build";
import { z as zod } from "zod";
import {
  runtimeInspectionSchema,
  type RuntimeInspection,
} from "@/types/capabilities";
import {
  huggingFaceRepositorySchema,
  modelCatalogSchema,
  modelDownloadEventSchema,
  modelDownloadOutcomeSchema,
  type HuggingFaceRepository,
  type ModelCatalog,
  type ModelDownloadEvent,
  type ModelDownloadOutcome,
  type ModelDownloadRequest,
  type ProjectorSelection,
} from "@/types/models";
import {
  commandPreviewSchema,
  launchProfileSchema,
  type CommandPreview,
  type LaunchProfile,
  type ProfileInput,
} from "@/types/profiles";
import {
  serverEventSchema,
  serverLogsSnapshotSchema,
  serverSnapshotSchema,
  type ServerEvent,
  type ServerLogsSnapshot,
  type ServerSnapshot,
} from "@/types/server";
import {
  performanceBenchmarkSchema,
  performancePlanSchema,
  performanceSweepEventSchema,
  performanceSweepSchema,
  type PerformanceBenchmark,
  type PerformancePlan,
  type PerformanceSweep,
  type PerformanceSweepEvent,
} from "@/types/performance";

/**
 * The only module that talks to Rust.
 *
 * Every backend call is declared here with its argument and result types, so a signature change
 * in Rust surfaces as a single TypeScript error rather than scattered runtime failures.
 */
async function call<Schema extends z.ZodType>(
  command: string,
  schema: Schema,
  args?: Record<string, unknown>,
): Promise<z.infer<Schema>> {
  let raw: unknown;
  try {
    raw = await invoke(command, args);
  } catch (error) {
    throw toAppError(error);
  }

  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    throw {
      code: "internal" as const,
      message: `The backend returned an unexpected shape for "${command}".`,
      hint: "This usually means the Rust and frontend versions are out of sync.",
      details: parsed.error.message,
    };
  }

  return parsed.data;
}

const voidSchema = zod.union([zod.null(), zod.undefined()]).transform(() => undefined);

export const ipc = {
  testAgentConnection: (
    request: AgentConnectionTestRequest,
  ): Promise<AgentConnectionTest> =>
    call("test_agent_connection", agentConnectionTestSchema, { request }),

  getAppInfo: (): Promise<AppInfo> => call("get_app_info", appInfoSchema),

  getHardwareSnapshot: (): Promise<HardwareSnapshot> =>
    call("get_hardware_snapshot", hardwareSnapshotSchema),

  getGitVersion: (): Promise<string> => call("get_git_version", zod.string()),

  revealPath: (path: string): Promise<void> =>
    call("reveal_path", voidSchema, { path }),

  getSettings: (): Promise<Settings> => call("get_settings", settingsSchema),

  updateSettings: (settings: Settings): Promise<Settings> =>
    call("update_settings", settingsSchema, { settings }),

  resetSettings: (): Promise<Settings> => call("reset_settings", settingsSchema),

  listSources: (): Promise<LlamaSource[]> =>
    call("list_sources", zod.array(llamaSourceSchema)),

  addExistingSource: (directory: string, name?: string): Promise<LlamaSource> =>
    call("add_existing_source", llamaSourceSchema, { directory, name: name ?? null }),

  cloneSource: (
    request: CloneRequest,
    onProgress: Channel<ProgressEvent>,
  ): Promise<LlamaSource> =>
    call("clone_source", llamaSourceSchema, { request, onProgress }),

  removeSource: (id: string, deleteDirectory: boolean): Promise<void> =>
    call("remove_source", voidSchema, { id, deleteDirectory }),

  getSourceStatus: (id: string): Promise<SourceStatus> =>
    call("get_source_status", sourceStatusSchema, { id }),

  fetchSource: (
    id: string,
    remote: string | null,
    onProgress: Channel<ProgressEvent>,
  ): Promise<void> => call("fetch_source", voidSchema, { id, remote, onProgress }),

  updateSource: (id: string): Promise<UpdateOutcome> =>
    call("update_source", updateOutcomeSchema, { id }),

  listSourceRefs: (id: string): Promise<SourceRefs> =>
    call("list_source_refs", sourceRefsSchema, { id }),

  switchSourceRef: (id: string, target: string): Promise<SwitchRefOutcome> =>
    call("switch_source_ref", switchRefOutcomeSchema, { id, target }),

  listSourceRemotes: (id: string): Promise<GitRemote[]> =>
    call("list_source_remotes", zod.array(gitRemoteSchema), { id }),

  addSourceRemote: (id: string, name: string, url: string): Promise<GitRemote[]> =>
    call("add_source_remote", zod.array(gitRemoteSchema), { id, name, url }),

  removeSourceRemote: (id: string, name: string): Promise<GitRemote[]> =>
    call("remove_source_remote", zod.array(gitRemoteSchema), { id, name }),

  discoverDefaultBranch: (repository: string): Promise<string | null> =>
    call("discover_default_branch", zod.string().nullable(), { repository }),

  detectToolchain: (): Promise<Toolchain> =>
    call("detect_toolchain", toolchainSchema),

  buildRuntime: (
    request: BuildRequest,
    onProgress: Channel<ProgressEvent>,
  ): Promise<BuildOutcome> =>
    call("build_runtime", buildOutcomeSchema, { request, onProgress }),

  cancelBuild: (): Promise<boolean> => call("cancel_build", zod.boolean()),

  isBuildRunning: (): Promise<boolean> => call("is_build_running", zod.boolean()),

  listRuntimes: (): Promise<RuntimeRecord[]> =>
    call("list_runtimes", zod.array(runtimeRecordSchema)),

  getRuntimeCapabilities: (id: string): Promise<RuntimeInspection> =>
    call("get_runtime_capabilities", runtimeInspectionSchema, { id }),

  inspectRuntimeCapabilities: (id: string): Promise<RuntimeInspection> =>
    call("inspect_runtime_capabilities", runtimeInspectionSchema, { id }),

  deleteRuntime: (id: string): Promise<void> =>
    call("delete_runtime", voidSchema, { id }),

  scanModels: (): Promise<ModelCatalog> => call("scan_models", modelCatalogSchema),

  setModelProjector: (
    modelId: string,
    selection: ProjectorSelection,
  ): Promise<void> =>
    call("set_model_projector", voidSchema, { modelId, selection }),

  inspectHuggingFaceRepository: (
    repository: string,
  ): Promise<HuggingFaceRepository> =>
    call("inspect_hugging_face_repository", huggingFaceRepositorySchema, {
      repository,
    }),

  downloadHuggingFaceModel: (
    request: ModelDownloadRequest,
    onEvent: Channel<ModelDownloadEvent>,
  ): Promise<ModelDownloadOutcome> =>
    call("download_hugging_face_model", modelDownloadOutcomeSchema, {
      request,
      onEvent,
    }),

  cancelModelDownload: (): Promise<boolean> =>
    call("cancel_model_download", zod.boolean()),

  isModelDownloadRunning: (): Promise<boolean> =>
    call("is_model_download_running", zod.boolean()),

  listProfiles: (): Promise<LaunchProfile[]> =>
    call("list_profiles", zod.array(launchProfileSchema)),

  createProfile: (input: ProfileInput): Promise<LaunchProfile> =>
    call("create_profile", launchProfileSchema, { input }),

  updateProfile: (id: string, input: ProfileInput): Promise<LaunchProfile> =>
    call("update_profile", launchProfileSchema, { id, input }),

  deleteProfile: (id: string): Promise<void> =>
    call("delete_profile", voidSchema, { id }),

  previewProfileCommand: (
    profileId: string | null,
    input: ProfileInput,
  ): Promise<CommandPreview> =>
    call("preview_profile_command", commandPreviewSchema, { profileId, input }),

  getPerformancePlan: (profileId: string): Promise<PerformancePlan> =>
    call("get_performance_plan", performancePlanSchema, { profileId }),

  listPerformanceBenchmarks: (
    profileId: string | null,
  ): Promise<PerformanceBenchmark[]> =>
    call("list_performance_benchmarks", zod.array(performanceBenchmarkSchema), {
      profileId,
    }),

  runPerformanceBenchmark: (profileId: string): Promise<PerformanceBenchmark> =>
    call("run_performance_benchmark", performanceBenchmarkSchema, { profileId }),

  runPerformanceSweep: (
    profileId: string,
    onEvent: Channel<PerformanceSweepEvent>,
  ): Promise<PerformanceSweep> =>
    call("run_performance_sweep", performanceSweepSchema, { profileId, onEvent }),

  cancelPerformanceSweep: (): Promise<boolean> =>
    call("cancel_performance_sweep", zod.boolean()),

  isPerformanceSweepRunning: (): Promise<boolean> =>
    call("is_performance_sweep_running", zod.boolean()),

  getServerStatus: (): Promise<ServerSnapshot> =>
    call("get_server_status", serverSnapshotSchema),

  getServerLogs: (): Promise<ServerLogsSnapshot> =>
    call("get_server_logs", serverLogsSnapshotSchema),

  subscribeServerEvents: (onEvent: Channel<ServerEvent>): Promise<string> =>
    call("subscribe_server_events", zod.string(), { onEvent }),

  unsubscribeServerEvents: (id: string): Promise<boolean> =>
    call("unsubscribe_server_events", zod.boolean(), { id }),

  clearServerLogs: (): Promise<void> => call("clear_server_logs", voidSchema),

  startServer: (profileId: string): Promise<ServerSnapshot> =>
    call("start_server", serverSnapshotSchema, { profileId }),

  stopServer: (): Promise<ServerSnapshot> =>
    call("stop_server", serverSnapshotSchema),

  restartServer: (): Promise<ServerSnapshot> =>
    call("restart_server", serverSnapshotSchema),
};

export function createProgressChannel(
  onEvent: (event: ProgressEvent) => void,
): Channel<ProgressEvent> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onEvent;
  return channel;
}

export function createPerformanceSweepChannel(
  onEvent: (event: PerformanceSweepEvent) => void,
): Channel<PerformanceSweepEvent> {
  const channel = new Channel<PerformanceSweepEvent>();
  channel.onmessage = (raw) => {
    const parsed = performanceSweepEventSchema.safeParse(raw);
    if (parsed.success) onEvent(parsed.data);
  };
  return channel;
}

export function createModelDownloadChannel(
  onEvent: (event: ModelDownloadEvent) => void,
): Channel<ModelDownloadEvent> {
  const channel = new Channel<ModelDownloadEvent>();
  channel.onmessage = (raw) => {
    const parsed = modelDownloadEventSchema.safeParse(raw);
    if (parsed.success) onEvent(parsed.data);
  };
  return channel;
}

export function createServerEventChannel(
  onEvent: (event: ServerEvent) => void,
): Channel<ServerEvent> {
  const channel = new Channel<ServerEvent>();
  channel.onmessage = (raw) => {
    const parsed = serverEventSchema.safeParse(raw);
    if (parsed.success) {
      onEvent(parsed.data);
    }
  };
  return channel;
}

export function hasTauriRuntime(): boolean {
  return isTauri();
}
