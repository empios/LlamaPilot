import {
  AlertTriangleIcon,
  CheckCircle2Icon,
  DownloadIcon,
  UnlinkIcon,
} from "lucide-react";

import { StatGrid, StatTile } from "@/components/stat-tile";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { formatCommit, formatRelativeTime, pluralize } from "@/lib/format";
import type { SourceStatus } from "@/types/sources";

export function SourceStatusSummary({ status }: { status: SourceStatus }) {
  const { git, head, source } = status;

  return (
    <div className="flex flex-col gap-5">
      <StatGrid>
        <StatTile
          label="Checked out ref"
          value={git.detached ? "Detached HEAD" : (git.branch ?? "unknown")}
          detail={git.upstream ? `tracking ${git.upstream}` : "no upstream branch"}
        />
        <StatTile
          label="Commit"
          mono
          value={formatCommit(git.commit)}
          detail={head?.subject}
        />
        <StatTile
          label="Upstream"
          value={
            git.behind > 0
              ? `${git.behind} behind`
              : git.ahead > 0
                ? `${git.ahead} ahead`
                : "In sync"
          }
          detail={
            git.ahead > 0 && git.behind > 0
              ? `${git.ahead} ahead · ${git.behind} behind`
              : "Fetch to re-check upstream"
          }
        />
        <StatTile
          label="Last fetch"
          value={formatRelativeTime(source.lastFetchedAt)}
          detail={head ? `HEAD by ${head.author}` : undefined}
        />
      </StatGrid>

      <WorktreeState status={status} />
    </div>
  );
}

function WorktreeState({ status }: { status: SourceStatus }) {
  const { git } = status;
  const trackedChanges = git.staged + git.unstaged + git.conflicted;

  if (trackedChanges > 0) {
    return (
      <Alert variant="destructive">
        <AlertTriangleIcon />
        <AlertTitle>
          {trackedChanges} uncommitted {pluralize(trackedChanges, "change")} in the working tree
        </AlertTitle>
        <AlertDescription className="flex flex-col gap-2">
          <span>
            Updating and switching refs are blocked while tracked files are modified. Nothing is
            stashed or discarded on your behalf — review the changes in Git first.
          </span>
          <span className="font-mono text-xs tabular-nums">
            {git.staged} staged · {git.unstaged} unstaged · {git.conflicted} conflicted ·{" "}
            {git.untracked} untracked
          </span>
        </AlertDescription>
      </Alert>
    );
  }

  if (git.detached) {
    return (
      <Alert>
        <UnlinkIcon />
        <AlertTitle>HEAD is detached</AlertTitle>
        <AlertDescription>
          Expected after checking out a tag or commit. The checkout is reproducible, but it
          cannot be updated until you switch back to a branch.
        </AlertDescription>
      </Alert>
    );
  }

  if (status.updateAvailable) {
    return (
      <Alert>
        <DownloadIcon />
        <AlertTitle>
          {git.behind} new upstream {pluralize(git.behind, "commit")} available
        </AlertTitle>
        <AlertDescription>
          {status.upstreamHead
            ? `Upstream is at ${status.upstreamHead.shortCommit} — ${status.upstreamHead.subject}`
            : "Updating fast-forwards the branch and never creates a merge commit."}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <Alert>
      <CheckCircle2Icon />
      <AlertTitle>Working tree is clean and up to date</AlertTitle>
      <AlertDescription>
        {git.untracked > 0
          ? `${git.untracked} untracked ${pluralize(git.untracked, "file")} present; these do not block updates.`
          : "Nothing to commit, and no upstream commits pending."}
      </AlertDescription>
    </Alert>
  );
}
