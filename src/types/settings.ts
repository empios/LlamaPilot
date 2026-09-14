import { z } from "zod";

export const themePreferenceSchema = z.enum(["system", "light", "dark"]);
export type ThemePreference = z.infer<typeof themePreferenceSchema>;

export const settingsSchema = z.object({
  updates: z.object({
    autoCheck: z.boolean(),
    autoDownload: z.boolean(),
    autoInstall: z.boolean(),
  }).default({ autoCheck: true, autoDownload: false, autoInstall: false }),
  schemaVersion: z.number().int(),
  appearance: z.object({
    theme: themePreferenceSchema,
    compactDensity: z.boolean(),
  }),
  workspace: z.object({
    sourcesDirectory: z.string().nullable(),
    buildsDirectory: z.string().nullable(),
    modelDirectories: z.array(z.string()),
  }),
  git: z.object({
    defaultRepository: z.string(),
    executable: z.string().nullable(),
  }),
  build: z.object({
    cmakeExecutable: z.string().nullable(),
    parallelJobs: z.number().int().nullable(),
  }),
  server: z.object({
    defaultHost: z.string(),
    defaultPort: z.number().int().min(1).max(65535),
    autoSelectPort: z.boolean(),
  }),
});

export type Settings = z.infer<typeof settingsSchema>;

export const appPathsSchema = z.object({
  dataDir: z.string(),
  settingsFile: z.string(),
  sourcesDir: z.string(),
  sourcesMetadataFile: z.string(),
  buildsDir: z.string(),
  buildsMetadataFile: z.string(),
  profilesDir: z.string(),
  runtimesDir: z.string(),
  modelsDir: z.string(),
  modelsMetadataFile: z.string(),
  cacheDir: z.string(),
  modelMetadataCacheFile: z.string(),
  logsDir: z.string(),
  defaultWorkspaceDir: z.string(),
});

export const appInfoSchema = z.object({
  name: z.string(),
  version: z.string(),
  paths: appPathsSchema,
  platform: z.string(),
});

export type AppInfo = z.infer<typeof appInfoSchema>;
export type AppPaths = z.infer<typeof appPathsSchema>;
