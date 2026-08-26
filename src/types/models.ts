import { z } from "zod";

export const ggufMetadataSchema = z.object({
  version: z.number().int(),
  tensorCount: z.number().int(),
  metadataCount: z.number().int(),
  metadataBytes: z.number().int(),
  generalType: z.string().nullable(),
  architecture: z.string().nullable(),
  name: z.string().nullable(),
  basename: z.string().nullable(),
  sizeLabel: z.string().nullable(),
  fileType: z.number().int().nullable(),
  quantizationVersion: z.number().int().nullable(),
  contextLength: z.number().int().nullable(),
  embeddingLength: z.number().int().nullable(),
  blockCount: z.number().int().nullable(),
  tokenizerModel: z.string().nullable(),
  splitIndex: z.number().int().nullable(),
  splitCount: z.number().int().nullable(),
  splitTensorCount: z.number().int().nullable(),
  projectorType: z.string().nullable(),
  hasVisionEncoder: z.boolean().nullable(),
  hasAudioEncoder: z.boolean().nullable(),
});

export const modelShardSchema = z.object({
  path: z.string(),
  index: z.number().int(),
  sizeBytes: z.number().int(),
});

export const projectorStatusSchema = z.enum([
  "none",
  "auto",
  "ambiguous",
  "overridden",
  "disabled",
  "missingOverride",
]);

export const modelRecordSchema = z.object({
  id: z.string(),
  displayName: z.string(),
  primaryPath: z.string().nullable(),
  directory: z.string(),
  shards: z.array(modelShardSchema),
  expectedShards: z.number().int(),
  complete: z.boolean(),
  totalSizeBytes: z.number().int(),
  modifiedAt: z.string(),
  metadata: ggufMetadataSchema,
  projectorStatus: projectorStatusSchema,
  projectorId: z.string().nullable(),
  projectorCandidates: z.array(z.string()),
});

export const projectorRecordSchema = z.object({
  id: z.string(),
  displayName: z.string(),
  path: z.string(),
  directory: z.string(),
  sizeBytes: z.number().int(),
  modifiedAt: z.string(),
  metadata: ggufMetadataSchema,
});

export const modelCatalogSchema = z.object({
  scannedAt: z.string(),
  roots: z.array(z.string()),
  models: z.array(modelRecordSchema),
  projectors: z.array(projectorRecordSchema),
  issues: z.array(
    z.object({
      path: z.string(),
      message: z.string(),
    }),
  ),
  cacheHits: z.number().int(),
  cacheMisses: z.number().int(),
});

export type GgufMetadata = z.infer<typeof ggufMetadataSchema>;
export type ModelRecord = z.infer<typeof modelRecordSchema>;
export type ProjectorRecord = z.infer<typeof projectorRecordSchema>;
export type ModelCatalog = z.infer<typeof modelCatalogSchema>;

export type ProjectorSelection =
  | { mode: "auto" }
  | { mode: "disabled" }
  | { mode: "custom"; path: string };
