import { useState } from "react";

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
import { Field, FieldLabel } from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import { useRemoveSource } from "@/hooks/use-sources";
import { useNavigationStore } from "@/stores/navigation-store";

interface RemoveSourceDialogProps {
  sourceId: string;
  sourceName: string;
  directory: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function RemoveSourceDialog({
  sourceId,
  sourceName,
  directory,
  open,
  onOpenChange,
}: RemoveSourceDialogProps) {
  const removeSource = useRemoveSource();
  const selectSource = useNavigationStore((state) => state.selectSource);
  const [deleteDirectory, setDeleteDirectory] = useState(false);

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Remove {sourceName}?</AlertDialogTitle>
          <AlertDialogDescription>
            The source is removed from this application. By default the folder on disk is left
            exactly as it is.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <Field orientation="horizontal">
          <Switch
            id="delete-directory"
            checked={deleteDirectory}
            onCheckedChange={setDeleteDirectory}
          />
          <FieldLabel htmlFor="delete-directory">
            Also delete {directory} permanently
          </FieldLabel>
        </Field>

        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={() =>
              removeSource.mutate(
                { id: sourceId, deleteDirectory },
                { onSuccess: () => selectSource(null) },
              )
            }
          >
            Remove source
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
