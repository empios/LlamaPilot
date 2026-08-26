import { PlusIcon } from "lucide-react";
import { useState } from "react";

import { ErrorPanel } from "@/components/error-panel";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { AddSourceDialog } from "@/features/runtime/add-source-dialog";
import { SourceDetail } from "@/features/runtime/source-detail";
import { SourceList } from "@/features/runtime/source-list";
import { useSources } from "@/hooks/use-sources";
import { useNavigationStore } from "@/stores/navigation-store";

export function RuntimesPage() {
  const sources = useSources();
  const requestedSourceId = useNavigationStore((state) => state.selectedSourceId);
  const selectSource = useNavigationStore((state) => state.selectSource);
  const [addDialogOpen, setAddDialogOpen] = useState(false);

  const availableSources = sources.data ?? [];

  // Resolved during render rather than in an effect: an effect would paint an empty detail pane
  // first and then immediately repaint with the selection, which reads as a flicker.
  const selectedSourceId =
    availableSources.find((source) => source.id === requestedSourceId)?.id ??
    availableSources[0]?.id ??
    null;

  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="llama.cpp"
        title="Runtimes"
        description="Clone sources, track upstream, switch branches or tags, and add experimental forks."
        actions={
          <Button onClick={() => setAddDialogOpen(true)}>
            <PlusIcon data-icon="inline-start" />
            Add source
          </Button>
        }
      />

      {sources.isError ? <ErrorPanel error={sources.error} /> : null}
      {sources.isPending ? <Skeleton className="h-72 w-full" /> : null}

      {sources.data ? (
        <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
          <SourceList
            sources={availableSources}
            selectedId={selectedSourceId}
            onSelect={selectSource}
            onAdd={() => setAddDialogOpen(true)}
          />
          {selectedSourceId ? <SourceDetail sourceId={selectedSourceId} /> : null}
        </div>
      ) : null}

      <AddSourceDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />
    </div>
  );
}
