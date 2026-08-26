import { CpuIcon, MemoryStickIcon, MicrochipIcon } from "lucide-react";

import { MetricBar } from "@/components/metric-bar";
import { Section } from "@/components/section";
import { StatGrid, StatTile } from "@/components/stat-tile";
import { Skeleton } from "@/components/ui/skeleton";
import { useHardwareSnapshot } from "@/hooks/use-app-info";
import { formatBytes, formatMebibytes, pluralize } from "@/lib/format";

export function HardwarePanel() {
  const hardware = useHardwareSnapshot();

  return (
    <Section
      label="Hardware"
      title="This machine"
      description="llama.cpp device names come from the selected runtime, not from here."
      icon={MicrochipIcon}
      bodyClassName="flex flex-col gap-6 p-4"
    >
      {hardware.data ? (
        <>
          <StatGrid className="lg:grid-cols-3">
            <StatTile
              icon={CpuIcon}
              label="Processor"
              value={hardware.data.cpu.brand}
              detail={`${hardware.data.cpu.logicalCores} logical${
                hardware.data.cpu.physicalCores
                  ? ` · ${hardware.data.cpu.physicalCores} physical`
                  : ""
              } ${pluralize(hardware.data.cpu.logicalCores, "core")}`}
            />
            <StatTile
              icon={MemoryStickIcon}
              label="System memory"
              mono
              value={formatBytes(hardware.data.memory.totalBytes)}
              detail={`${formatBytes(hardware.data.memory.availableBytes)} available`}
            />
            <StatTile
              icon={MicrochipIcon}
              label="NVIDIA driver"
              mono
              value={hardware.data.nvidiaDriver ?? "Not detected"}
              detail={
                hardware.data.nvidiaPresent
                  ? `${hardware.data.gpus.length} ${pluralize(hardware.data.gpus.length, "GPU")} detected`
                  : "nvidia-smi did not report a GPU"
              }
            />
          </StatGrid>

          {hardware.data.gpus.length > 0 ? (
            <div className="flex flex-col gap-4 border-t border-border pt-5">
              <h3 className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
                Video memory
              </h3>
              {hardware.data.gpus.map((gpu) => (
                <MetricBar
                  key={gpu.index}
                  label={`GPU ${gpu.index} · ${gpu.name}`}
                  used={gpu.usedMemoryMib}
                  total={gpu.totalMemoryMib}
                  formatValue={formatMebibytes}
                  sublabel={`${formatMebibytes(gpu.freeMemoryMib)} free`}
                />
              ))}
            </div>
          ) : null}
        </>
      ) : (
        <div className="flex flex-col gap-4">
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      )}
    </Section>
  );
}
