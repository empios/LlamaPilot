import type { GitRef } from "@/types/sources";

/** `origin/experimental/dflash` → `experimental/dflash`. */
export function localBranchName(remoteBranch: string): string {
  const separator = remoteBranch.indexOf("/");
  return separator === -1 ? remoteBranch : remoteBranch.slice(separator + 1);
}

/**
 * Presents local and remote branches as one list of branches.
 *
 * Whether a branch already exists locally is plumbing: the backend decides between checking out
 * a local branch and creating a tracking branch. Showing two separate lists made the user answer
 * a question the app already answers, and listed `master` twice.
 *
 * The checked-out branch sorts first, then the rest alphabetically, with remote-only branches
 * after the ones already checked out locally.
 */
export function mergeBranches(
  localBranches: GitRef[],
  remoteBranches: GitRef[],
): GitRef[] {
  const localNames = new Set(localBranches.map((branch) => branch.name));

  const remoteOnly = remoteBranches.filter(
    (branch) => !localNames.has(localBranchName(branch.name)),
  );

  const byName = (left: GitRef, right: GitRef) => left.name.localeCompare(right.name);

  return [
    ...localBranches.filter((branch) => branch.isHead),
    ...localBranches.filter((branch) => !branch.isHead).sort(byName),
    ...remoteOnly.sort(byName),
  ];
}
