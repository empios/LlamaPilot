/**
 * Central registry of TanStack Query keys.
 *
 * Declaring them once keeps invalidation honest: a mutation cannot invalidate a key that does
 * not exist, and renaming a key is a single edit.
 */
export const queryKeys = {
  appInfo: ["app-info"] as const,
  hardware: ["hardware"] as const,
  gitVersion: ["git-version"] as const,
  settings: ["settings"] as const,
  toolchain: ["toolchain"] as const,
  buildStatus: ["build-status"] as const,
  runtimes: ["runtimes"] as const,
  runtimeCapabilities: (id: string) => ["runtimes", id, "capabilities"] as const,
  models: ["models"] as const,
  profiles: ["profiles"] as const,
  serverStatus: ["server", "status"] as const,
  serverLogs: ["server", "logs"] as const,
  performancePlan: (profileId: string) => ["performance", "plan", profileId] as const,
  performanceBenchmarks: (profileId: string) =>
    ["performance", "benchmarks", profileId] as const,
  performanceSweepStatus: ["performance", "sweep", "status"] as const,
  profilePreview: (profileId: string | null, input: unknown) =>
    ["profiles", "preview", profileId ?? "new", input] as const,
  sources: ["sources"] as const,
  sourceStatus: (id: string) => ["sources", id, "status"] as const,
  sourceRefs: (id: string) => ["sources", id, "refs"] as const,
  sourceRemotes: (id: string) => ["sources", id, "remotes"] as const,
};
