import { CheckIcon, MonitorIcon, MoonIcon, SunIcon, TriangleAlertIcon } from "lucide-react";

import { StatusChip } from "@/components/status-chip";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useGitVersion, useHardwareSnapshot } from "@/hooks/use-app-info";
import { useSettings, useUpdateSettings } from "@/hooks/use-settings";
import { useThemeStore } from "@/stores/theme-store";
import { themePreferenceSchema, type ThemePreference } from "@/types/settings";

const themeOptions = [
  { value: "light", label: "Light", icon: SunIcon },
  { value: "dark", label: "Dark", icon: MoonIcon },
  { value: "system", label: "System", icon: MonitorIcon },
] as const;

/**
 * Environment status strip.
 *
 * It deliberately does not repeat the page title: the sidebar shows where you are and the page
 * header owns the heading, so duplicating it here just wasted a row.
 */
export function AppTopbar() {
  return (
    <header className="flex h-14 shrink-0 items-center justify-end gap-5 border-b border-border px-6">
      <div className="flex items-center gap-2">
        <GitStatus />
        <GpuStatus />
      </div>

      <div className="flex items-center gap-2 border-l border-border pl-5">
        <span className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
          Theme
        </span>
        <ThemeToggle />
      </div>
    </header>
  );
}

function GitStatus() {
  const gitVersion = useGitVersion();

  if (gitVersion.isPending) {
    return <StatusChip tone="neutral" label="Git" value="checking" />;
  }

  if (gitVersion.isError) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <StatusChip
            tone="danger"
            label="Git"
            value="not found"
            icon={TriangleAlertIcon}
          />
        </TooltipTrigger>
        <TooltipContent>
          Install Git for Windows, or set an explicit path in Settings.
        </TooltipContent>
      </Tooltip>
    );
  }

  const version = gitVersion.data?.replace("git version ", "") ?? "unknown";

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <StatusChip tone="success" label="Git" value={version} icon={CheckIcon} />
      </TooltipTrigger>
      <TooltipContent>{gitVersion.data}</TooltipContent>
    </Tooltip>
  );
}

function GpuStatus() {
  const hardware = useHardwareSnapshot();

  if (!hardware.data) {
    return null;
  }

  const count = hardware.data.gpus.length;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <StatusChip
          tone={count > 0 ? "success" : "neutral"}
          label="GPU"
          value={count > 0 ? `${count} NVIDIA` : "none detected"}
        />
      </TooltipTrigger>
      <TooltipContent>
        {count > 0
          ? hardware.data.gpus.map((gpu) => gpu.name).join(", ")
          : "nvidia-smi did not report a GPU."}
      </TooltipContent>
    </Tooltip>
  );
}

function ThemeToggle() {
  const preference = useThemeStore((state) => state.preference);
  const setPreference = useThemeStore((state) => state.setPreference);
  const settings = useSettings();
  const updateSettings = useUpdateSettings();

  const applyTheme = (next: ThemePreference) => {
    setPreference(next);

    if (settings.data && settings.data.appearance.theme !== next) {
      updateSettings.mutate({
        ...settings.data,
        appearance: { ...settings.data.appearance, theme: next },
      });
    }
  };

  return (
    <ToggleGroup
      type="single"
      size="sm"
      variant="outline"
      spacing={0}
      value={preference}
      onValueChange={(value) => {
        const parsed = themePreferenceSchema.safeParse(value);
        if (parsed.success) {
          applyTheme(parsed.data);
        }
      }}
    >
      {themeOptions.map((option) => {
        const Icon = option.icon;
        return (
          <Tooltip key={option.value}>
            <TooltipTrigger asChild>
              <ToggleGroupItem value={option.value} aria-label={option.label}>
                <Icon />
              </ToggleGroupItem>
            </TooltipTrigger>
            <TooltipContent>{option.label}</TooltipContent>
          </Tooltip>
        );
      })}
    </ToggleGroup>
  );
}
