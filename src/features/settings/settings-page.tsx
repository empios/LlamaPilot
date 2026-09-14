import { UpdateSettings } from "@/features/settings/update-settings";
import { RotateCcwIcon } from "lucide-react";

import { ErrorPanel } from "@/components/error-panel";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useResetSettings, useSettings } from "@/hooks/use-settings";
import { GeneralSettings } from "@/features/settings/general-settings";
import { PathsSettings } from "@/features/settings/paths-settings";
import { ToolingSettings } from "@/features/settings/tooling-settings";

export function SettingsPage() {
  const settings = useSettings();
  const resetSettings = useResetSettings();

  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="System"
        title="Settings"
        description="Stored as human-readable JSON in the application data directory."
        actions={
          <Button
            variant="outline"
            onClick={() => resetSettings.mutate()}
            disabled={resetSettings.isPending}
          >
            <RotateCcwIcon data-icon="inline-start" />
            Restore defaults
          </Button>
        }
      />

      {settings.isPending ? <Skeleton className="h-64 w-full" /> : null}
      {settings.isError ? <ErrorPanel error={settings.error} /> : null}

      {settings.data ? (
        <Tabs defaultValue="general">
          <TabsList>
            <TabsTrigger value="general">General</TabsTrigger>
            <TabsTrigger value="paths">Paths</TabsTrigger>
            <TabsTrigger value="tooling">Tooling</TabsTrigger>
            <TabsTrigger value="updates">Updates</TabsTrigger>
          </TabsList>
          <TabsContent value="updates">
            <UpdateSettings settings={settings.data} />
          </TabsContent>
          <TabsContent value="general">
            <GeneralSettings settings={settings.data} />
          </TabsContent>
          <TabsContent value="paths">
            <PathsSettings settings={settings.data} />
          </TabsContent>
          <TabsContent value="tooling">
            <ToolingSettings settings={settings.data} />
          </TabsContent>
        </Tabs>
      ) : null}
    </div>
  );
}
