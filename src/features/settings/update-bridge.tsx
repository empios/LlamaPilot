import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { useAppUpdate, useUpdateActions } from "@/hooks/use-app-update";
import { useSettings } from "@/hooks/use-settings";
import { hasUpdateBlockers, useUpdateStore } from "@/stores/update-store";
import { useNavigationStore } from "@/stores/navigation-store";

const DAY = 24 * 60 * 60 * 1000;

/** One scheduler in the app shell; settings tabs do not own background updates. */
export function UpdateBridge() {
  const { data: status } = useAppUpdate();
  const { data: settings } = useSettings();
  const action = useUpdateActions();
  const { countdown, working, error } = useUpdateStore();
  const navigate = useNavigationStore((s) => s.navigate);
  const lastAttempt = useRef(0);
  const downloadAttempt = useRef<string | null>(null);
  const [seconds, setSeconds] = useState(30);

  const latest = useRef({ status, settings });
  latest.current = { status, settings };

  useEffect(() => {
    const tick = () => {
      const { status, settings } = latest.current;
      if (!status || !settings || useUpdateStore.getState().working) return;
      const prefs = settings.updates;
      const now = Date.now();
      const last = Math.max(lastAttempt.current, status.lastChecked ? Date.parse(status.lastChecked) : 0);
      if (status.canCheck && prefs.autoCheck && now - last >= DAY && !["ready", "downloading", "installing"].includes(status.phase)) {
        lastAttempt.current = now;
        void action("check");
        return;
      }
      const deferred = status.version !== null && status.deferredVersion === status.version;
      if (status.canInstall && status.phase === "available" && !deferred && (prefs.autoDownload || prefs.autoInstall) && status.version !== downloadAttempt.current) {
        downloadAttempt.current = status.version;
        void action("download");
        return;
      }
      const ready = status.phase === "ready" && status.canInstall && prefs.autoInstall
        && !deferred && !status.busy && !hasUpdateBlockers() && !useUpdateStore.getState().error
        && document.visibilityState === "visible";
      if (!ready) {
        if (useUpdateStore.getState().countdown !== null) useUpdateStore.setState({ countdown: null });
        return;
      }
      const due = useUpdateStore.getState().countdown;
      if (due === null) {
        useUpdateStore.setState({ countdown: now + 30_000 });
        setSeconds(30);
      } else {
        setSeconds(Math.max(0, Math.ceil((due - now) / 1000)));
        if (now >= due) void action("install", true);
      }
    };
    // Use a timer rather than a React lifecycle to invoke actions that flush the UI.
    const timer = window.setInterval(tick, 1000);
    return () => window.clearInterval(timer);
  }, [action]);

  if (!status?.version || status.deferredVersion === status.version) return null;
  return <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border bg-card px-6 py-2 text-sm" role="status">
    <span>{countdown !== null ? `LlamaPilot will update and restart in ${seconds}s.` : `LlamaPilot ${status.version} is available.${status.phase === "ready" ? " Ready to install when your work is finished." : ""}`}{error ? ` ${error}` : ""}</span>
    <div className="flex gap-2">
      <Button size="sm" variant="outline" onClick={() => navigate("settings")}>Updates in Settings</Button>
      <Button size="sm" variant="ghost" disabled={working} onClick={() => void action("defer")}>Later</Button>
    </div>
  </div>;
}
