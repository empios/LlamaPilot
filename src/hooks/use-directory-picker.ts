import { open } from "@tauri-apps/plugin-dialog";
import { useCallback } from "react";
import { toast } from "sonner";

/**
 * Opens the native folder picker.
 *
 * The picker only yields a path string; nothing is executed and no filesystem scope is granted
 * to the webview by selecting a folder.
 */
export function useDirectoryPicker() {
  return useCallback(async (title: string): Promise<string | null> => {
    try {
      const selected = await open({ directory: true, multiple: false, title });
      return typeof selected === "string" ? selected : null;
    } catch (error) {
      toast.error("Could not open the folder picker", {
        description: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  }, []);
}
