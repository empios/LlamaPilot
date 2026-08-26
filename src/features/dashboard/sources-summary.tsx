import { ChevronRightIcon, GitBranchIcon, PlusIcon } from "lucide-react";

import { ErrorPanel } from "@/components/error-panel";
import { Section } from "@/components/section";
import { SourceIdentity } from "@/features/runtime/source-identity";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useSources } from "@/hooks/use-sources";
import { formatRelativeTime, pluralize } from "@/lib/format";
import { useNavigationStore } from "@/stores/navigation-store";

export function SourcesSummary() {
  const sources = useSources();
  const navigate = useNavigationStore((state) => state.navigate);
  const selectSource = useNavigationStore((state) => state.selectSource);

  const count = sources.data?.length ?? 0;

  return (
    <Section
      label="Sources"
      title="llama.cpp working copies"
      description={
        count > 0
          ? `${count} ${pluralize(count, "source")} registered`
          : "Nothing registered yet"
      }
      icon={GitBranchIcon}
      actions={
        <Button variant="outline" size="sm" onClick={() => navigate("runtimes")}>
          Open Runtimes
        </Button>
      }
      bodyClassName={count > 0 ? "p-0" : "p-4"}
    >
      {sources.isPending ? <Skeleton className="h-12 w-full" /> : null}
      {sources.isError ? <ErrorPanel error={sources.error} /> : null}

      {sources.data && count === 0 ? (
        <div className="flex flex-col items-start gap-3">
          <p className="text-sm text-muted-foreground">
            Clone <code className="font-mono">ggml-org/llama.cpp</code>, or register a checkout
            you already have.
          </p>
          <Button size="sm" onClick={() => navigate("runtimes")}>
            <PlusIcon data-icon="inline-start" />
            Add source
          </Button>
        </div>
      ) : null}

      {sources.data && count > 0 ? (
        <ul className="divide-y divide-border">
          {sources.data.map((source) => (
            <li key={source.id}>
              <button
                type="button"
                onClick={() => {
                  selectSource(source.id);
                  navigate("runtimes");
                }}
                className="flex w-full items-center justify-between gap-4 px-4 py-3 text-left transition-colors duration-100 hover:bg-accent/50 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none focus-visible:-outline-offset-2"
              >
                <SourceIdentity source={source} />
                <span className="flex shrink-0 items-center gap-3 text-xs text-muted-foreground">
                  <span className="hidden sm:inline">
                    fetched {formatRelativeTime(source.lastFetchedAt)}
                  </span>
                  <ChevronRightIcon className="size-4" />
                </span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </Section>
  );
}
