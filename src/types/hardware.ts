import { z } from "zod";

export const gpuInfoSchema = z.object({
  index: z.number().int(),
  name: z.string(),
  totalMemoryMib: z.number(),
  usedMemoryMib: z.number(),
  freeMemoryMib: z.number(),
  driverVersion: z.string().nullish(),
});

export const hardwareSnapshotSchema = z.object({
  cpu: z.object({
    brand: z.string(),
    physicalCores: z.number().int().nullish(),
    logicalCores: z.number().int(),
  }),
  memory: z.object({
    totalBytes: z.number(),
    availableBytes: z.number(),
  }),
  gpus: z.array(gpuInfoSchema),
  nvidiaDriver: z.string().nullish(),
  nvidiaPresent: z.boolean(),
});

export type GpuInfo = z.infer<typeof gpuInfoSchema>;
export type HardwareSnapshot = z.infer<typeof hardwareSnapshotSchema>;
