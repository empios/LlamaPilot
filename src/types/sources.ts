import { z } from "zod";

export const gitRefKindSchema = z.enum(["localBranch", "remoteBranch", "tag"]);
export type GitRefKind = z.infer<typeof gitRefKindSchema>;

export const gitRefSchema = z.object({
  name: z.string(),
  fullName: z.string(),
  kind: gitRefKindSchema,
  commit: z.string(),
  shortCommit: z.string(),
  createdAt: z.string().nullish(),
  remote: z.string().nullish(),
  upstream: z.string().nullish(),
  isHead: z.boolean(),
});

export const commitInfoSchema = z.object({
  commit: z.string(),
  shortCommit: z.string(),
  committedAt: z.string(),
  author: z.string(),
  subject: z.string(),
});

export const gitStatusSchema = z.object({
  commit: z.string().nullish(),
  branch: z.string().nullish(),
  detached: z.boolean(),
  upstream: z.string().nullish(),
  ahead: z.number().int(),
  behind: z.number().int(),
  staged: z.number().int(),
  unstaged: z.number().int(),
  untracked: z.number().int(),
  conflicted: z.number().int(),
});

export const gitRemoteSchema = z.object({
  name: z.string(),
  fetchUrl: z.string().nullish(),
  pushUrl: z.string().nullish(),
});

export const llamaSourceSchema = z.object({
  id: z.string(),
  name: z.string(),
  repository: z.string(),
  directory: z.string(),
  remote: z.string(),
  currentRef: z.string(),
  currentCommit: z.string(),
  addedAt: z.string(),
  lastFetchedAt: z.string().nullish(),
});

export const sourceStatusSchema = z.object({
  source: llamaSourceSchema,
  directoryExists: z.boolean(),
  git: gitStatusSchema,
  head: commitInfoSchema.nullish(),
  upstreamHead: commitInfoSchema.nullish(),
  remotes: z.array(gitRemoteSchema),
  updateAvailable: z.boolean(),
});

export const sourceRefsSchema = z.object({
  localBranches: z.array(gitRefSchema),
  remoteBranches: z.array(gitRefSchema),
  tags: z.array(gitRefSchema),
});

export const updateOutcomeSchema = z.object({
  previousCommit: z.string().nullish(),
  currentCommit: z.string().nullish(),
  changed: z.boolean(),
});

export const refCheckoutSchema = z.enum([
  "localBranch",
  "trackRemoteBranch",
  "detach",
]);

export const switchRefOutcomeSchema = z.object({
  source: llamaSourceSchema,
  checkout: refCheckoutSchema,
  detached: z.boolean(),
});

export type GitRef = z.infer<typeof gitRefSchema>;
export type CommitInfo = z.infer<typeof commitInfoSchema>;
export type GitStatus = z.infer<typeof gitStatusSchema>;
export type GitRemote = z.infer<typeof gitRemoteSchema>;
export type LlamaSource = z.infer<typeof llamaSourceSchema>;
export type SourceStatus = z.infer<typeof sourceStatusSchema>;
export type SourceRefs = z.infer<typeof sourceRefsSchema>;
export type UpdateOutcome = z.infer<typeof updateOutcomeSchema>;
export type RefCheckout = z.infer<typeof refCheckoutSchema>;
export type SwitchRefOutcome = z.infer<typeof switchRefOutcomeSchema>;

export interface CloneRequest {
  repository: string;
  destinationParent?: string | null;
  directoryName?: string | null;
  branch?: string | null;
  name?: string | null;
}

/** Streamed output from a long-running Git operation. */
export type ProgressEvent =
  | { kind: "started"; label: string }
  | { kind: "output"; stream: "stdout" | "stderr"; text: string }
  | { kind: "finished"; success: boolean };
