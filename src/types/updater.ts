import { z } from "zod";

export const updateStatusSchema = z.object({
  phase: z.enum(["idle", "checking", "unavailable", "available", "downloading", "ready", "installing", "error"]),
  currentVersion: z.string(),
  version: z.string().nullable(),
  notes: z.string().nullable(),
  lastChecked: z.string().nullable(),
  downloaded: z.number(),
  total: z.number().nullable(),
  canCheck: z.boolean(),
  canInstall: z.boolean(),
  reason: z.string().nullable(),
  error: z.string().nullable(),
  deferredVersion: z.string().nullable(),
  busy: z.boolean(),
  releasesUrl: z.string(),
});
export type UpdateStatus = z.infer<typeof updateStatusSchema>;
