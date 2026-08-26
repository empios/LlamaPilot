/**
 * Single source of truth for user-visible product naming.
 *
 * Renaming the application means editing this file, `src-tauri/tauri.conf.json`, and the crate
 * name in `src-tauri/Cargo.toml` — nothing else references the name directly.
 */
export const branding = {
  name: "Llama Control",
  shortName: "Control",
  tagline: "Control panel for llama.cpp",
  upstreamRepository: "https://github.com/ggml-org/llama.cpp",
} as const;
