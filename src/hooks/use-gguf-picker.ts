import { open } from "@tauri-apps/plugin-dialog";
import { useCallback } from "react";
import { toast } from "sonner";

export function useGgufPicker() {
  return useCallback(async (): Promise<string | null> => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: "Choose a multimodal projector",
        filters: [{ name: "GGUF projector", extensions: ["gguf"] }],
      });
      return typeof selected === "string" ? selected : null;
    } catch (error) {
      toast.error("Could not open the file picker", {
        description: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  }, []);
}
