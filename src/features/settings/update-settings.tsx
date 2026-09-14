import { DownloadIcon } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useAppUpdate, useUpdateActions } from "@/hooks/use-app-update";
import { useUpdateSettings } from "@/hooks/use-settings";
import { useUpdateStore } from "@/stores/update-store";
import { showAppError } from "@/lib/toast-error";
import type { Settings } from "@/types/settings";

export function UpdateSettings({ settings }: { settings: Settings }) {
  const update = useAppUpdate();
  const action = useUpdateActions();
  const save = useUpdateSettings();
  const { working, error } = useUpdateStore();
  const status = update.data;
  const updatePreference = (key: keyof Settings["updates"], value: boolean) => {
    save.mutate({ ...settings, updates: { ...settings.updates, [key]: value } });
  };
  return (
    <Section label="Updates" title="Application updates" icon={DownloadIcon}
      description="Keep LlamaPilot up to date. Your models, runtimes and profiles are preserved."
      bodyClassName="flex flex-col gap-5 p-4">
      <div className="flex flex-wrap gap-6 text-sm">
        <span>Installed: <strong>{status?.currentVersion ?? "—"}</strong></span>
        <span>Available: <strong>{status?.version ?? "—"}</strong></span>
        <span>Last checked: {status?.lastChecked ? new Date(status.lastChecked).toLocaleString() : "Never"}</span>
      </div>
      {status?.reason && <p className="text-sm text-muted-foreground">{status.reason}</p>}
      {!status && <p className="text-sm">{update.isError ? "Could not read update status. Try reopening this page." : "Update checking is available in the desktop app."}</p>}
      <div className="flex flex-col gap-4">
        {([
          ["autoCheck", "Check automatically", "Check at startup and every 24 hours while LlamaPilot is open."],
          ["autoDownload", "Download in the background", "Download verified updates when they become available."],
          ["autoInstall", "Install automatically when idle", "Allows LlamaPilot to download updates and restart after work finishes, with a 30-second countdown. Active servers and open editors delay installation."],
        ] as const).map(([key, title, description]) => (
          <div className="flex items-start justify-between gap-6" key={key}>
            <label htmlFor={`update-${key}`} className="text-sm"><span className="font-medium">{title}</span><span className="mt-1 block text-muted-foreground">{description}</span></label>
            <Switch id={`update-${key}`} checked={settings.updates[key]} onCheckedChange={(value) => updatePreference(key, value)} disabled={save.isPending || working || (key !== "autoCheck" && !status?.canInstall)} />
          </div>
        ))}
      </div>
      {status?.notes && <p className="max-h-48 overflow-auto whitespace-pre-wrap text-sm">{status.notes}</p>}
      {status?.phase === "downloading" && <div role="status" className="text-sm">
        Downloading: {(status.downloaded / 1_048_576).toFixed(1)} MB
        {status.total ? ` of ${(status.total / 1_048_576).toFixed(1)} MB` : " (total size unknown)"}
        <progress className="mt-2 w-full" value={status.total ? status.downloaded : undefined} max={status.total ?? undefined} aria-label="Update download" />
      </div>}
      {status?.busy && status.version && <p className="text-sm">Installation will wait until the server and other work have stopped.</p>}
      {(error || status?.error) && <p role="alert" className="text-sm text-destructive">{error ?? status?.error}</p>}
      {status?.phase === "unavailable" && <p role="status" className="text-sm">
        Update service unavailable. The release server did not provide update information.
        Try again later or check Release downloads. Your installed app can still be used.
      </p>}
      {status?.phase === "idle" && status.lastChecked && !status.version && <p role="status" className="text-sm">You are up to date.</p>}
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" disabled={working || !status?.canCheck} onClick={() => void action("check")}>{status?.phase === "checking" ? "Checking…" : "Check now"}</Button>
        {status?.version && status.canInstall && status.phase !== "ready" && <Button disabled={working} onClick={() => void action("download")}>Download update</Button>}
        {status?.phase === "ready" && <Button disabled={working || status.busy} onClick={() => void action("install")}>Install and restart</Button>}
        {status?.version && <Button variant="ghost" disabled={working} onClick={() => void action("defer")}>Later</Button>}
        <Button variant="outline" onClick={() => void openUrl("https://github.com/empios/LlamaPilot/releases/latest").catch(showAppError)}>Release downloads</Button>
      </div>
    </Section>
  );
}
