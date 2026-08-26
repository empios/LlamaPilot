import { z } from "zod";

export const llamaOptionCategorySchema = z.enum([
  "context",
  "batching",
  "gpu",
  "memory",
  "parallelism",
  "templates",
  "reasoning",
  "speculative",
  "advanced",
]);

export const llamaOptionSchema = z.object({
  flag: z.string(),
  aliases: z.array(z.string()),
  valueHint: z.string().nullable(),
  description: z.string(),
  section: z.string(),
  category: llamaOptionCategorySchema,
  knownKey: z.string().nullable(),
  displayName: z.string(),
  summary: z.string().nullable(),
});

export const llamaDeviceSchema = z.object({
  id: z.string(),
  name: z.string(),
  backend: z.string().nullable(),
  memoryTotalMib: z.number().int().nonnegative().nullable(),
  memoryFreeMib: z.number().int().nonnegative().nullable(),
  raw: z.string(),
});

export const llamaCapabilitiesSchema = z.object({
  schemaVersion: z.number().int().positive(),
  version: z.string(),
  commit: z.string().nullable(),
  options: z.record(z.string(), llamaOptionSchema),
  speculativeTypes: z.array(z.string()),
  devices: z.array(llamaDeviceSchema),
});

export const rawCommandOutputSchema = z.object({
  stdout: z.string(),
  stderr: z.string(),
});

export const llamaRawOutputsSchema = z.object({
  version: rawCommandOutputSchema,
  help: rawCommandOutputSchema,
  devices: rawCommandOutputSchema,
});

export const runtimeInspectionSchema = z.object({
  capabilities: llamaCapabilitiesSchema,
  raw: llamaRawOutputsSchema,
});

export const runtimeCapabilitySummarySchema = z.object({
  inspectionId: z.string(),
  inspectedAt: z.string(),
  version: z.string(),
  commit: z.string().nullable(),
  optionCount: z.number().int().nonnegative(),
  knownOptionCount: z.number().int().nonnegative(),
  deviceCount: z.number().int().nonnegative(),
  speculativeTypeCount: z.number().int().nonnegative(),
});

export type LlamaOptionCategory = z.infer<typeof llamaOptionCategorySchema>;
export type LlamaOption = z.infer<typeof llamaOptionSchema>;
export type LlamaDevice = z.infer<typeof llamaDeviceSchema>;
export type LlamaCapabilities = z.infer<typeof llamaCapabilitiesSchema>;
export type RawCommandOutput = z.infer<typeof rawCommandOutputSchema>;
export type RuntimeInspection = z.infer<typeof runtimeInspectionSchema>;
export type RuntimeCapabilitySummary = z.infer<
  typeof runtimeCapabilitySummarySchema
>;
