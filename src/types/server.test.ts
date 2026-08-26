import { describe, expect, it } from "vitest";

import {
  serverEventSchema,
  serverIsActive,
  serverSnapshotSchema,
  serverStateLabel,
} from "@/types/server";

const stopped = {
  generation: 0,
  state: "stopped",
  pid: null,
  profileId: null,
  profileName: null,
  runtimeId: null,
  runtimeLabel: null,
  modelName: null,
  host: null,
  port: null,
  startedAt: null,
  stoppedAt: null,
  exitCode: null,
  healthStatus: null,
  healthMessage: null,
  lastError: null,
  logFile: null,
  telemetry: {
    propsAvailable: null,
    slotsAvailable: null,
    metricsAvailable: null,
    totalSlots: null,
    busySlots: null,
    requestsProcessing: null,
    requestsDeferred: null,
    promptTokensPerSecond: null,
    predictedTokensPerSecond: null,
    buildInfo: null,
  },
} as const;

describe("server IPC payloads", () => {
  it("accepts the complete backend snapshot", () => {
    expect(serverSnapshotSchema.parse(stopped).state).toBe("stopped");
    expect(serverEventSchema.parse({ kind: "status", snapshot: stopped }).kind).toBe("status");
  });

  it("treats every transitional process state as active", () => {
    expect(serverIsActive("starting")).toBe(true);
    expect(serverIsActive("loading")).toBe(true);
    expect(serverIsActive("ready")).toBe(true);
    expect(serverIsActive("busy")).toBe(true);
    expect(serverIsActive("stopping")).toBe(true);
    expect(serverIsActive("stopped")).toBe(false);
    expect(serverIsActive("crashed")).toBe(false);
  });

  it("formats lifecycle values for compact status badges", () => {
    expect(serverStateLabel("starting")).toBe("Starting");
    expect(serverStateLabel("crashed")).toBe("Crashed");
  });
});
