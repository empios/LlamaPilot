import { z } from "zod";

export const profileOptionSettingSchema = z.discriminatedUnion("mode", [
  z.object({ mode: z.literal("default") }),
  z.object({ mode: z.literal("auto") }),
  z.object({ mode: z.literal("custom"), value: z.string() }),
]);

export const profileInputSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  runtimeId: z.string(),
  modelId: z.string(),
  host: z.string(),
  port: z.number().int().min(0).max(65535),
  autoSelectPort: z.boolean(),
  options: z.record(z.string(), profileOptionSettingSchema),
  environment: z.record(z.string(), z.string()),
  additionalArguments: z.array(z.string()),
});

export const launchProfileSchema = profileInputSchema.extend({
  schemaVersion: z.number().int().positive(),
  id: z.string(),
  runtimeLabel: z.string(),
  modelName: z.string(),
  modelPath: z.string(),
  projectorPath: z.string().nullable(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export const commandPreviewSchema = z.object({
  program: z.string(),
  arguments: z.array(z.string()),
  environment: z.record(z.string(), z.string()),
  plain: z.string(),
  powershell: z.string(),
  posix: z.string().optional(),
  runtimeLabel: z.string(),
  modelName: z.string(),
  capabilityVersion: z.string(),
  warnings: z.array(z.string()),
});

export type ProfileOptionSetting = z.infer<typeof profileOptionSettingSchema>;
export type ProfileInput = z.infer<typeof profileInputSchema>;
export type LaunchProfile = z.infer<typeof launchProfileSchema>;
export type CommandPreview = z.infer<typeof commandPreviewSchema>;

export function createProfileInput({
  host,
  port,
  autoSelectPort,
  runtimeId = "",
  modelId = "",
}: {
  host: string;
  port: number;
  autoSelectPort: boolean;
  runtimeId?: string;
  modelId?: string;
}): ProfileInput {
  return {
    name: "New profile",
    description: null,
    runtimeId,
    modelId,
    host,
    port,
    autoSelectPort,
    options: {},
    environment: {},
    additionalArguments: [],
  };
}

export function profileToInput(profile: LaunchProfile): ProfileInput {
  return {
    name: profile.name,
    description: profile.description,
    runtimeId: profile.runtimeId,
    modelId: profile.modelId,
    host: profile.host,
    port: profile.port,
    autoSelectPort: profile.autoSelectPort,
    options: profile.options,
    environment: profile.environment,
    additionalArguments: profile.additionalArguments,
  };
}
