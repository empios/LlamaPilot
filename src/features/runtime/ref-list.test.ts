import { describe, expect, it } from "vitest";

import { localBranchName, mergeBranches } from "./ref-list";
import type { GitRef } from "@/types/sources";

function branch(
  name: string,
  kind: GitRef["kind"],
  options: Partial<GitRef> = {},
): GitRef {
  return {
    name,
    fullName: `refs/${kind}/${name}`,
    kind,
    commit: "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
    shortCommit: "a1b2c3d",
    createdAt: "2026-08-24T10:11:12+02:00",
    remote: kind === "remoteBranch" ? name.split("/")[0]! : null,
    upstream: null,
    isHead: false,
    ...options,
  };
}

describe("localBranchName", () => {
  it("drops the remote prefix", () => {
    expect(localBranchName("origin/master")).toBe("master");
    expect(localBranchName("experimental/gg/dflash")).toBe("gg/dflash");
  });

  it("leaves a bare name alone", () => {
    expect(localBranchName("master")).toBe("master");
  });
});

describe("mergeBranches", () => {
  it("does not list a branch twice when it exists locally and remotely", () => {
    const merged = mergeBranches(
      [branch("master", "localBranch", { isHead: true })],
      [branch("origin/master", "remoteBranch")],
    );

    expect(merged.map((reference) => reference.name)).toEqual(["master"]);
  });

  it("keeps remote-only branches so forks are reachable", () => {
    const merged = mergeBranches(
      [branch("master", "localBranch", { isHead: true })],
      [
        branch("origin/master", "remoteBranch"),
        branch("experimental/dflash-wip", "remoteBranch"),
      ],
    );

    expect(merged.map((reference) => reference.name)).toEqual([
      "master",
      "experimental/dflash-wip",
    ]);
  });

  it("puts the checked-out branch first", () => {
    const merged = mergeBranches(
      [
        branch("alpha", "localBranch"),
        branch("zulu", "localBranch", { isHead: true }),
        branch("mike", "localBranch"),
      ],
      [],
    );

    expect(merged.map((reference) => reference.name)).toEqual(["zulu", "alpha", "mike"]);
  });

  it("lists local branches before remote-only ones", () => {
    const merged = mergeBranches(
      [branch("zulu", "localBranch")],
      [branch("origin/alpha", "remoteBranch")],
    );

    expect(merged.map((reference) => reference.name)).toEqual(["zulu", "origin/alpha"]);
  });

  it("handles an empty repository", () => {
    expect(mergeBranches([], [])).toEqual([]);
  });
});
