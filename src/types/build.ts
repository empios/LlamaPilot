import { z } from "zod";

import { runtimeCapabilitySummarySchema } from "@/types/capabilities";

export const buildBackendSchema = z.enum(["cpu", "cuda", "metal"]);
export const buildConfigurationSchema = z.enum([
  "release",
  "relWithDebInfo",
  "debug",
]);

export const toolIdSchema = z.enum([
  "git",
  "cmake",
  "visualStudio",
  "msvc",
  "cxx",
  "make",
  "ninja",
  "cudaToolkit",
  "nvcc",
  "nvidiaDriver",
]);

export const toolRequirementSchema = z.enum([
  "required",
  "requiredForCuda",
  "optional",
]);

export const toolStatusSchema = z.object({
  id: toolIdSchema,
  name: z.string(),
  found: z.boolean(),
  version: z.string().nullish(),
  path: z.string().nullish(),
  detail: z.string().nullish(),
  requirement: toolRequirementSchema,
  remedy: z.string().nullish(),
});

export const cmakeGeneratorSchema = z.object({
  name: z.string(),
  isDefault: z.boolean(),
  multiConfig: z.boolean(),
});

export const toolchainSchema = z.object({
  tools: z.array(toolStatusSchema),
  generators: z.array(cmakeGeneratorSchema),
  backends: z.array(buildBackendSchema),
  defaultBackend: buildBackendSchema,
});

export const buildProfileSchema = z.object({
  backend: buildBackendSchema,
  configuration: buildConfigurationSchema,
  generator: z.string().nullable(),
  parallelJobs: z.number().int().nullable(),
  nativeOptimizations: z.boolean(),
  cudaArchitectures: z.string().nullable(),
  additionalCmakeArgs: z.array(z.string()),
});

export const runtimeRecordSchema = z.object({
  id: z.string(),
  sourceId: z.string(),
  sourceName: z.string(),
  repository: z.string(),
  commit: z.string(),
  shortCommit: z.string(),
  branch: z.string(),
  backend: buildBackendSchema,
  configuration: buildConfigurationSchema,
  generator: z.string(),
  buildDate: z.string(),
  directory: z.string(),
  executable: z.string(),
  sizeBytes: z.number(),
  fileCount: z.number().int(),
  capabilities: runtimeCapabilitySummarySchema.nullable(),
});

export const buildOutcomeSchema = z.object({
  runtime: runtimeRecordSchema,
  buildDirectory: z.string(),
  reconfigured: z.boolean(),
});

export type BuildBackend = z.infer<typeof buildBackendSchema>;
export type BuildConfiguration = z.infer<typeof buildConfigurationSchema>;
export type ToolId = z.infer<typeof toolIdSchema>;
export type ToolRequirement = z.infer<typeof toolRequirementSchema>;
export type ToolStatus = z.infer<typeof toolStatusSchema>;
export type CmakeGenerator = z.infer<typeof cmakeGeneratorSchema>;
export type Toolchain = z.infer<typeof toolchainSchema>;
export type BuildProfile = z.infer<typeof buildProfileSchema>;
export type RuntimeRecord = z.infer<typeof runtimeRecordSchema>;
export type BuildOutcome = z.infer<typeof buildOutcomeSchema>;

export interface BuildRequest {
  sourceId: string;
  profile: BuildProfile;
  clean: boolean;
}

export const defaultBuildProfile: BuildProfile = {
  backend: "cpu",
  configuration: "release",
  generator: null,
  parallelJobs: null,
  nativeOptimizations: true,
  cudaArchitectures: null,
  additionalCmakeArgs: [],
};

/** Mirrors `BuildBackend::label` in Rust. */
export function backendLabel(backend: BuildBackend): string {
  switch (backend) {
    case "cuda":
      return "CUDA";
    case "metal":
      return "Metal";
    case "cpu":
      return "CPU";
    default: {
      const exhaustive: never = backend;
      return exhaustive;
    }
  }
}

export function configurationLabel(configuration: BuildConfiguration): string {
  switch (configuration) {
    case "release":
      return "Release";
    case "relWithDebInfo":
      return "RelWithDebInfo";
    case "debug":
      return "Debug";
    default: {
      const exhaustive: never = configuration;
      return exhaustive;
    }
  }
}
