import {
  FolderOpenIcon,
  PencilIcon,
  PlayIcon,
  PlusIcon,
  RotateCwIcon,
  SlidersHorizontalIcon,
  SquareIcon,
  TrashIcon,
} from "lucide-react";
import { useState } from "react";

import { ErrorPanel } from "@/components/error-panel";
import { PageHeader } from "@/components/page-header";
import { Section } from "@/components/section";
import { StatusChip } from "@/components/status-chip";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { ProfileEditor } from "@/features/profiles/profile-editor";
import { useRuntimes } from "@/hooks/use-build";
import { useModels } from "@/hooks/use-models";
import { useDeleteProfile, useProfiles } from "@/hooks/use-profiles";
import { useSettings } from "@/hooks/use-settings";
import { useAppInfo } from "@/hooks/use-app-info";
import {
  useRestartServer,
  useServerStatus,
  useStartServer,
  useStopServer,
} from "@/hooks/use-server";
import { formatTimestamp, pluralize } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import type { LaunchProfile } from "@/types/profiles";
import { serverIsActive, serverStateLabel } from "@/types/server";

type EditorState = { key: number; profile: LaunchProfile | null };

export function ProfilesPage() {
  const profiles = useProfiles();
  const runtimes = useRuntimes();
  const models = useModels();
  const settings = useSettings();
  const appInfo = useAppInfo();
  const deleteProfile = useDeleteProfile();
  const server = useServerStatus();
  const startServer = useStartServer();
  const stopServer = useStopServer();
  const restartServer = useRestartServer();
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [pendingDelete, setPendingDelete] = useState<LaunchProfile | null>(null);

  const loading = profiles.isPending || runtimes.isPending || models.isPending || settings.isPending;
  const profileCount = profiles.data?.length ?? 0;
  const firstError = profiles.error ?? runtimes.error ?? models.error ?? settings.error ?? server.error;
  const serverActive = server.data ? serverIsActive(server.data.state) : false;

  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="Serve"
        title="Profiles"
        description="Reproducible launch configurations pinned to one model and immutable runtime."
        actions={
          <>
            {appInfo.data ? (
              <Button
                variant="outline"
                onClick={() => void ipc.revealPath(appInfo.data.paths.profilesDir)}
              >
                <FolderOpenIcon data-icon="inline-start" />
                Profile files
              </Button>
            ) : null}
            <Button onClick={() => setEditor({ key: Date.now(), profile: null })}>
              <PlusIcon data-icon="inline-start" />
              New profile
            </Button>
          </>
        }
      />

      {loading ? <Skeleton className="h-48 w-full" /> : null}
      {firstError ? <ErrorPanel error={firstError} /> : null}

      {!loading && !firstError && profiles.data && runtimes.data && models.data && settings.data ? (
        <>
          <div className="flex flex-wrap gap-2">
            <StatusChip label="Profiles" value={String(profileCount)} />
            <StatusChip label="Runtimes" value={String(runtimes.data.length)} />
            <StatusChip label="Complete models" value={String(models.data.models.filter((model) => model.complete).length)} />
            <StatusChip
              label="Inspected runtimes"
              value={String(runtimes.data.filter((runtime) => runtime.capabilities).length)}
            />
          </div>

          <Section
            label="Launch profiles"
            title={`${profileCount} saved ${pluralize(profileCount, "profile")}`}
            description="Each entry is a separate readable JSON document."
            icon={SlidersHorizontalIcon}
            bodyClassName={profileCount > 0 ? "p-0" : "p-4"}
          >
            {profileCount === 0 ? (
              <Empty>
                <EmptyHeader>
                  <EmptyMedia variant="icon">
                    <SlidersHorizontalIcon />
                  </EmptyMedia>
                  <EmptyTitle>No launch profiles yet</EmptyTitle>
                  <EmptyDescription>
                    Choose a model and inspected runtime, then configure only the values you want to override.
                  </EmptyDescription>
                </EmptyHeader>
                <EmptyContent>
                  <Button onClick={() => setEditor({ key: Date.now(), profile: null })}>
                    Create first profile
                  </Button>
                </EmptyContent>
              </Empty>
            ) : (
              <ul className="divide-y divide-border">
                {profiles.data.map((profile) => (
                  <li
                    key={profile.id}
                    className="flex flex-wrap items-center justify-between gap-4 px-4 py-3"
                  >
                    <div className="flex min-w-0 flex-1 flex-col gap-1">
                      <span className="flex flex-wrap items-center gap-2">
                        <span className="truncate text-sm font-semibold">{profile.name}</span>
                        <Badge variant="secondary">{profile.modelName}</Badge>
                        {profile.projectorPath ? <Badge variant="outline">mmproj</Badge> : null}
                        {serverActive && server.data?.profileId === profile.id ? (
                          <Badge variant="outline">{serverStateLabel(server.data.state)}</Badge>
                        ) : null}
                      </span>
                      <span className="truncate text-xs text-muted-foreground">
                        {profile.runtimeLabel} · {profile.host}:{profile.port} · updated {formatTimestamp(profile.updatedAt)}
                      </span>
                      {profile.description ? (
                        <span className="truncate text-xs text-muted-foreground">{profile.description}</span>
                      ) : null}
                    </div>
                    <div className="flex shrink-0 items-center gap-1">
                      {serverActive && server.data?.profileId === profile.id ? (
                        <>
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={restartServer.isPending || server.data.state === "stopping"}
                            onClick={() => restartServer.mutate()}
                          >
                            <RotateCwIcon data-icon="inline-start" />
                            Restart
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={stopServer.isPending || server.data.state === "stopping"}
                            onClick={() => stopServer.mutate()}
                          >
                            <SquareIcon data-icon="inline-start" />
                            Stop
                          </Button>
                        </>
                      ) : (
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={startServer.isPending || serverActive}
                          title={serverActive ? "Stop the active profile first" : undefined}
                          onClick={() => startServer.mutate(profile.id)}
                        >
                          <PlayIcon data-icon="inline-start" />
                          Start
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setEditor({ key: Date.now(), profile })}
                      >
                        <PencilIcon data-icon="inline-start" />
                        Edit
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        disabled={
                          deleteProfile.isPending ||
                          (serverActive && server.data?.profileId === profile.id)
                        }
                        onClick={() => setPendingDelete(profile)}
                      >
                        <TrashIcon />
                        <span className="sr-only">Delete {profile.name}</span>
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </Section>

          {editor ? (
            <ProfileEditor
              key={`${editor.profile?.id ?? "new"}-${editor.key}`}
              profile={editor.profile}
              settings={settings.data}
              runtimes={runtimes.data}
              catalog={models.data}
              onClose={() => setEditor(null)}
            />
          ) : null}
        </>
      ) : null}

      <AlertDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete this launch profile?</AlertDialogTitle>
            <AlertDialogDescription>
              {pendingDelete
                ? `“${pendingDelete.name}” will be removed. Its model and runtime files are not affected.`
                : "The profile JSON file will be removed."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              disabled={deleteProfile.isPending}
              onClick={() => {
                if (pendingDelete) {
                  deleteProfile.mutate(pendingDelete.id, {
                    onSuccess: () => setPendingDelete(null),
                  });
                }
              }}
            >
              Delete profile
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
