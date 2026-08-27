import { z } from "zod";

export const agentConnectionTestSchema = z.object({
  apiBaseUrl: z.string(),
  modelIds: z.array(z.string()),
  selectedModel: z.string(),
  requestedModelMatched: z.boolean(),
  responseText: z.string(),
  modelsLatencyMs: z.number().nonnegative(),
  chatLatencyMs: z.number().nonnegative(),
});

export type AgentConnectionTest = z.infer<typeof agentConnectionTestSchema>;

export interface AgentConnectionTestRequest {
  apiKey: string | null;
  model: string | null;
}
