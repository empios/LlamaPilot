import { GitBranchIcon, PlusIcon } from "lucide-react";

import { SourceIdentity } from "@/features/runtime/source-identity";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { cn } from "@/lib/utils";
import type { LlamaSource } from "@/types/sources";

interface SourceListProps {
  sources: LlamaSource[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
}

export function SourceList({ sources, selectedId, onSelect, onAdd }: SourceListProps) {
  if (sources.length === 0) {
    return (
      <Empty className="rounded-lg border border-dashed border-border">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <GitBranchIcon />
          </EmptyMedia>
          <EmptyTitle>No llama.cpp source yet</EmptyTitle>
          <EmptyDescription>
            Clone the official repository, or register a checkout you already have.
          </EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button onClick={onAdd}>
            <PlusIcon data-icon="inline-start" />
            Add source
          </Button>
        </EmptyContent>
      </Empty>
    );
  }

  return (
    <nav
      aria-label="llama.cpp sources"
      className="overflow-hidden rounded-lg border border-border bg-card"
    >
      <h2 className="flex items-center justify-between gap-2 border-b border-border px-4 py-3 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
        Sources
        <span className="font-mono">{sources.length}</span>
      </h2>
      <ul className="divide-y divide-border">
        {sources.map((source) => {
          const isSelected = source.id === selectedId;

          return (
            <li key={source.id}>
              <button
                type="button"
                onClick={() => onSelect(source.id)}
                aria-current={isSelected ? "true" : undefined}
                className={cn(
                  "relative flex w-full items-center px-4 py-3 text-left",
                  "transition-colors duration-100",
                  "focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none focus-visible:-outline-offset-2",
                  isSelected ? "bg-accent" : "hover:bg-accent/50",
                )}
              >
                {isSelected ? (
                  <span
                    aria-hidden
                    className="absolute inset-y-0 left-0 w-0.5 bg-primary"
                  />
                ) : null}
                <SourceIdentity source={source} />
              </button>
            </li>
          );
        })}
      </ul>
      <div className="border-t border-border p-2">
        <Button variant="ghost" size="sm" className="w-full justify-start" onClick={onAdd}>
          <PlusIcon data-icon="inline-start" />
          Add source
        </Button>
      </div>
    </nav>
  );
}
