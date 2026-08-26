import { describe, expect, it } from "vitest";

import { formatBytes, formatCommit, formatTimestamp, pluralize } from "./format";

describe("formatBytes", () => {
  it("uses the largest unit that keeps the number readable", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1024 ** 3 * 24)).toBe("24.0 GB");
  });

  it("treats missing and negative sizes as zero", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(-1)).toBe("0 B");
    expect(formatBytes(Number.NaN)).toBe("0 B");
  });
});

describe("formatCommit", () => {
  it("shortens a full SHA to seven characters", () => {
    expect(formatCommit("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678")).toBe("a1b2c3d");
  });

  it("renders a dash when there is no commit", () => {
    expect(formatCommit(null)).toBe("—");
    expect(formatCommit(undefined)).toBe("—");
  });
});

describe("formatTimestamp", () => {
  it("returns the raw value when it is not a date", () => {
    expect(formatTimestamp("not-a-date")).toBe("not-a-date");
  });

  it("renders a dash when there is no value", () => {
    expect(formatTimestamp(null)).toBe("—");
  });
});

describe("pluralize", () => {
  it("keeps the singular for exactly one", () => {
    expect(pluralize(1, "commit")).toBe("commit");
    expect(pluralize(2, "commit")).toBe("commits");
    expect(pluralize(0, "shard")).toBe("shards");
  });
});
