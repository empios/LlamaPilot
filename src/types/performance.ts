import { z } from "zod";

import { profileOptionSettingSchema } from "@/types/profiles";

export const performanceDeviceSchema = z.object({
  id: z.string(),
  name: z.string(),
  totalMemoryMib: z.number().int().nullable(),
  freeMemoryMib: z.number().int().nullable(),
  splitPercent: z.number().int().min(0).max(100),
});

export const performanceCandidateSchema = z.object({
  id: z.string(),
  label: z.string(),
  description: z.string(),
  splitMode: z.string(),
  experimental: z.boolean(),
  options: z.record(z.string(), profileOptionSettingSchema),
  reasons: z.array(z.string()),
  warnings: z.array(z.string()),
});

export const performancePlanSchema = z.object({
  profileId: z.string(),
  profileName: z.string(),
  runtimeId: z.string(),
  runtimeLabel: z.string(),
  modelId: z.string(),
  modelName: z.string(),
  modelSizeBytes: z.number().int().nullable(),
  objective: z.literal("interactiveCoding"),
  devices: z.array(performanceDeviceSchema),
  totalFreeMemoryMib: z.number().int(),
  candidates: z.array(performanceCandidateSchema),
  recommendedCandidateId: z.string().nullable(),
  warnings: z.array(z.string()),
});

export const benchmarkPlacementSchema = z.object({
  devices: z.string().nullable(),
  splitMode: z.string().nullable(),
  tensorSplit: z.string().nullable(),
  gpuLayers: z.string().nullable(),
});

export const performanceBenchmarkSchema = z.object({
  schemaVersion: z.number().int().positive(),
  id: z.string(),
  createdAt: z.string(),
  profileId: z.string(),
  profileName: z.string(),
  runtimeId: z.string(),
  runtimeLabel: z.string(),
  modelName: z.string(),
  placement: benchmarkPlacementSchema,
  gpus: z.array(
    z.object({
      index: z.number().int(),
      name: z.string(),
      totalMemoryMib: z.number().int(),
      freeMemoryMib: z.number().int(),
    }),
  ),
  latencyMs: z.number(),
  promptTokens: z.number().int(),
  predictedTokens: z.number().int(),
  promptMs: z.number(),
  predictedMs: z.number(),
  promptTokensPerSecond: z.number(),
  predictedTokensPerSecond: z.number(),
  truncated: z.boolean(),
  stopType: z.string().nullable(),
  draftTokens: z.number().int().nullable(),
  acceptedDraftTokens: z.number().int().nullable(),
  sweepId: z.string().nullable(),
  candidateId: z.string().nullable(),
  candidateLabel: z.string().nullable(),
});

export const performanceSweepCandidateResultSchema = z.object({
  candidateId: z.string(),
  candidateLabel: z.string(),
  success: z.boolean(),
  benchmark: performanceBenchmarkSchema.nullable(),
  error: z.string().nullable(),
});

export const performanceSweepSchema = z.object({
  id: z.string(),
  profileId: z.string(),
  profileName: z.string(),
  startedAt: z.string(),
  finishedAt: z.string(),
  results: z.array(performanceSweepCandidateResultSchema),
  winnerCandidateId: z.string().nullable(),
  winnerBenchmarkId: z.string().nullable(),
  appliedWinner: z.boolean(),
});

export const performanceSweepEventSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("started"),
    sweepId: z.string(),
    totalCandidates: z.number().int(),
  }),
  z.object({
    kind: z.literal("candidateStarted"),
    candidateId: z.string(),
    candidateLabel: z.string(),
    index: z.number().int(),
    totalCandidates: z.number().int(),
  }),
  z.object({
    kind: z.literal("candidateFinished"),
    result: performanceSweepCandidateResultSchema,
    completedCandidates: z.number().int(),
    totalCandidates: z.number().int(),
  }),
  z.object({
    kind: z.literal("applyingWinner"),
    candidateId: z.string(),
    candidateLabel: z.string(),
  }),
  z.object({
    kind: z.literal("finished"),
    success: z.boolean(),
    winnerCandidateId: z.string().nullable(),
  }),
]);

export type PerformanceDevice = z.infer<typeof performanceDeviceSchema>;
export type PerformanceCandidate = z.infer<typeof performanceCandidateSchema>;
export type PerformancePlan = z.infer<typeof performancePlanSchema>;
export type PerformanceBenchmark = z.infer<typeof performanceBenchmarkSchema>;
export type PerformanceSweepCandidateResult = z.infer<
  typeof performanceSweepCandidateResultSchema
>;
export type PerformanceSweep = z.infer<typeof performanceSweepSchema>;
export type PerformanceSweepEvent = z.infer<typeof performanceSweepEventSchema>;
