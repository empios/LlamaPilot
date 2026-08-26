import { GitBranchIcon, LockIcon, SearchIcon, TagIcon, TerminalIcon } from "lucide-react";
import { useMemo, useState } from "react";

import { ErrorPanel } from "@/components/error-panel";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useSourceRefs, useSwitchSourceRef } from "@/hooks/use-sources";
import { formatTimestamp } from "@/lib/format";
import { cn } from "@/lib/utils";
import { localBranchName, mergeBranches } from "@/features/runtime/ref-list";
import type { GitRef } from "@/types/sources";

/**
 * llama.cpp publishes a build tag per merge, so a real checkout carries thousands of refs.
 * Rendering them all would stall the UI, so the list is capped and the filter does the work.
 */
const MAX_VISIBLE_REFS = 100;

type RefKind = "branches" | "tags";

interface SwitchRefPanelProps {
  sourceId: string;
  /** Set when switching cannot succeed, so the reason is shown instead of a failing button. */
  blockedReason?: string;
}

export function SwitchRefPanel({ sourceId, blockedReason }: SwitchRefPanelProps) {
  const refs = useSourceRefs(sourceId);
  const switchRef = useSwitchSourceRef();
  const [kind, setKind] = useState<RefKind>("branches");
  const [filter, setFilter] = useState("");

  const branches = useMemo(
    () => mergeBranches(refs.data?.localBranches ?? [], refs.data?.remoteBranches ?? []),
    [refs.data],
  );

  if (refs.isPending) {
    return <Skeleton className="h-64 w-full" />;
  }

  if (refs.isError) {
    return <ErrorPanel error={refs.error} />;
  }

  const tags = refs.data.tags;
  const pool = kind === "branches" ? branches : tags;
  const needle = filter.trim().toLowerCase();
  const matches = needle
    ? pool.filter((reference) => reference.name.toLowerCase().includes(needle))
    : pool;

  const visible = matches.slice(0, MAX_VISIBLE_REFS);
  const hidden = matches.length - visible.length;
  const switchingDisabled = switchRef.isPending || Boolean(blockedReason);
  const switchTo = (target: string) => switchRef.mutate({ id: sourceId, target });

  return (
    <div className="flex flex-col">
      {blockedReason ? (
        <Alert className="mb-4">
          <LockIcon />
          <AlertTitle>Switching is unavailable</AlertTitle>
          <AlertDescription>{blockedReason}</AlertDescription>
        </Alert>
      ) : null}

      <div className="flex flex-wrap items-center gap-2 pb-3">
        <InputGroup className="min-w-56 flex-1">
          <InputGroupAddon>
            <SearchIcon />
          </InputGroupAddon>
          <InputGroupInput
            value={filter}
            placeholder={`Filter ${pool.length.toLocaleString()} ${kind}`}
            onChange={(event) => setFilter(event.currentTarget.value)}
            aria-label={`Filter ${kind}`}
          />
        </InputGroup>

        <ToggleGroup
          type="single"
          size="sm"
          variant="outline"
          spacing={0}
          value={kind}
          onValueChange={(value) => {
            if (value === "branches" || value === "tags") {
              setKind(value);
            }
          }}
        >
          <ToggleGroupItem value="branches">
            <GitBranchIcon />
            Branches
            <span className="font-mono text-xs opacity-70">
              {branches.length.toLocaleString()}
            </span>
          </ToggleGroupItem>
          <ToggleGroupItem value="tags">
            <TagIcon />
            Tags
            <span className="font-mono text-xs opacity-70">
              {tags.length.toLocaleString()}
            </span>
          </ToggleGroupItem>
        </ToggleGroup>

        <AdvancedRefPopover disabled={switchingDisabled} onSwitch={switchTo} />
      </div>

      <div className="overflow-hidden rounded-md border border-border">
        {visible.length === 0 ? (
          <p className="px-4 py-10 text-center text-sm text-muted-foreground">
            {needle
              ? `No ${kind} match "${filter.trim()}".`
              : `No ${kind} yet. Fetch to see what the remotes publish.`}
          </p>
        ) : (
          // A plain scroll container rather than Radix ScrollArea: its viewport is laid out as
          // a table, which sizes to content and so defeats `truncate` on long branch names.
          <ul className="max-h-80 divide-y divide-border overflow-y-auto">
            {visible.map((reference) => (
              <RefRow
                key={reference.fullName}
                reference={reference}
                disabled={switchingDisabled}
                onSwitch={switchTo}
              />
            ))}
          </ul>
        )}
      </div>

      {hidden > 0 ? (
        <p className="pt-2 text-xs text-muted-foreground">
          Showing the first {visible.length} of {matches.length.toLocaleString()}. Type to
          narrow the list.
        </p>
      ) : null}
    </div>
  );
}

interface RefRowProps {
  reference: GitRef;
  disabled: boolean;
  onSwitch: (target: string) => void;
}

/**
 * A ref and the consequence of choosing it.
 *
 * The subtitle spells out what will happen — track a remote branch, or detach HEAD — so the
 * outcome is never a surprise.
 */
function RefRow({ reference, disabled, onSwitch }: RefRowProps) {
  const isTag = reference.kind === "tag";
  const isRemoteOnly = reference.kind === "remoteBranch";
  const Icon = isTag ? TagIcon : GitBranchIcon;

  const consequence = isTag
    ? "checks out a detached HEAD"
    : isRemoteOnly
      ? `creates local branch ${localBranchName(reference.name)}`
      : reference.upstream
        ? `tracks ${reference.upstream}`
        : "no upstream branch";

  return (
    <li
      className={cn(
        "flex items-center justify-between gap-3 px-3 py-2.5",
        reference.isHead && "bg-accent/40",
      )}
    >
      <div className="flex min-w-0 flex-1 items-start gap-2.5">
        <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="flex items-center gap-2">
            <span className="truncate text-sm font-medium">{reference.name}</span>
            {reference.isHead ? <Badge variant="secondary">current</Badge> : null}
            {isRemoteOnly && reference.remote ? (
              <Badge variant="outline" className="font-normal">
                {reference.remote}
              </Badge>
            ) : null}
          </span>
          <span className="truncate text-xs text-muted-foreground">
            <code className="font-mono">{reference.shortCommit}</code>
            {reference.createdAt ? ` · ${formatTimestamp(reference.createdAt)}` : null}
            {` · ${consequence}`}
          </span>
        </div>
      </div>

      <Button
        variant="outline"
        size="sm"
        className="shrink-0"
        disabled={disabled || reference.isHead}
        onClick={() => onSwitch(reference.name)}
      >
        {isTag ? "Check out" : "Switch"}
      </Button>
    </li>
  );
}

function AdvancedRefPopover({
  disabled,
  onSwitch,
}: {
  disabled: boolean;
  onSwitch: (target: string) => void;
}) {
  const [value, setValue] = useState("");
  const [open, setOpen] = useState(false);

  const submit = () => {
    const target = value.trim();
    if (!target) {
      return;
    }
    onSwitch(target);
    setOpen(false);
    setValue("");
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm">
          <TerminalIcon data-icon="inline-start" />
          Any ref
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <div className="flex flex-col gap-2">
          <label htmlFor="advanced-ref" className="text-sm font-medium">
            Branch, tag, or commit
          </label>
          <div className="flex gap-2">
            <Input
              id="advanced-ref"
              value={value}
              placeholder="origin/gg/experimental"
              onChange={(event) => setValue(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  submit();
                }
              }}
            />
            <Button disabled={!value.trim() || disabled} onClick={submit}>
              Switch
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Anything Git can resolve. A tag or commit checks out a detached HEAD on purpose.
          </p>
        </div>
      </PopoverContent>
    </Popover>
  );
}
