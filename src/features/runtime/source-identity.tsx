import { GitBranchIcon, UnlinkIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { formatCommit } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { LlamaSource } from "@/types/sources";

/**
 * Name, ref, and commit for a source, rendered identically wherever a source is listed so the
 * same information always appears in the same place.
 */
export function SourceIdentity({
  source,
  className,
}: {
  source: LlamaSource;
  className?: string;
}) {
  const isDetached = source.currentRef.startsWith("detached");
  const RefIcon = isDetached ? UnlinkIcon : GitBranchIcon;

  return (
    <span className={cn("flex min-w-0 flex-col gap-1", className)}>
      <span className="truncate text-sm font-medium">{source.name}</span>
      <span className="flex min-w-0 items-center gap-2">
        <Badge variant="outline" className="gap-1 font-normal">
          <RefIcon className="size-3" />
          <span className="truncate">
            {isDetached ? "detached" : source.currentRef}
          </span>
        </Badge>
        <code className="font-mono text-xs text-muted-foreground">
          {formatCommit(source.currentCommit)}
        </code>
      </span>
    </span>
  );
}
