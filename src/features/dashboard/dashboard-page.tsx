import { ServerIcon } from "lucide-react";

import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { HardwarePanel } from "@/features/dashboard/hardware-panel";
import { ServerPanel } from "@/features/dashboard/server-panel";
import { SourcesSummary } from "@/features/dashboard/sources-summary";
import { useNavigationStore } from "@/stores/navigation-store";

export function DashboardPage() {
  const navigate = useNavigationStore((state) => state.navigate);

  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="Serve"
        title="Dashboard"
        description="Current llama.cpp sources, build state, and the hardware they will run on."
        actions={
          <Button onClick={() => navigate("runtimes")}>
            <ServerIcon data-icon="inline-start" />
            Manage runtimes
          </Button>
        }
      />

      <ServerPanel />
      <SourcesSummary />
      <HardwarePanel />
    </div>
  );
}
