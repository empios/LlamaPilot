# Changelog

Notable changes to LlamaPilot are documented here. The project follows semantic versioning while
it is practical; before 1.0, minor releases may contain intentional breaking changes to persisted
development data.

## 0.4.0 — 2026-09-14

### Added

- Pangolin-based Paper and Terminal themes, bundled Ubuntu Sans typography, and
  refreshed navigation and shared desktop controls.
- Application updates in Settings: automatic stable-release checks, optional background
  downloads and opt-in installation when idle, with a visible restart countdown and deferral.
- Signed Tauri update artifacts and a complete GitHub Releases updater manifest, validated
  against build provenance and Minisign signatures before publication.
- A shared installation/work gate, unsaved-editor protection and single-instance startup.
  NSIS, installed macOS applications and writable AppImages support in-app installation;
  MSI, DEB and standalone executables retain manual package updates.

### Fixed

- Missing or unavailable update feeds now show a retryable service status instead of
  a generic application-update failure, without incorrectly reporting the app as up to date.

## 0.3.0 — 2026-09-10

### Added

- macOS Apple Silicon/Intel DMGs and Linux x64 AppImage/Debian packaging, with a native CI matrix
  and one complete GitHub Release containing all supported installers and SHA-256 checksums.
- Apple Metal builds, native Unix compiler detection, desktop tool lookup, Apple unified-memory
  reporting, POSIX command previews, and POSIX Aider connection snippets.
- Unix process-group watchdogs for cancellation and parent-exit cleanup, with integration tests.
- Relocatable static runtime defaults, versioned shared-library and Metal resource snapshots,
  platform support documentation, and portable version validation.


- A Performance Lab that generates runtime-valid all-GPU layer, row, and experimental tensor
  placement candidates for a saved profile.
- VRAM-weighted tensor-split recommendations with safety reserves and model-fit warnings.
- A repeatable coding benchmark using llama-server completion timings, with persistent per-profile
  history for comparing prompt processing, generation throughput, and end-to-end latency.
- An automatic tuning sweep that validates, starts, measures, and stops every generated candidate,
  skips failed placements, supports cancellation, restores the original profile on failure, and
  saves the fastest successful generation configuration.
- A public Hugging Face GGUF downloader with exact file selection, split-shard grouping, pinned
  revisions, progress and cancellation, safe temporary writes, and automatic catalog refresh.
- Agent Connect with supervised-profile launch, effective endpoint and model discovery, an actual
  OpenAI-compatible models/chat test, transient API-key support, and copy-ready JSON, OpenAI
  JavaScript, Aider PowerShell, OpenCode, and Pi configurations.

### Changed

- Redesigned the complete frontend around the MIT-licensed Pangolin Design System, including its
  Paper and Terminal themes, Ubuntu Sans typography, aubergine application shell, orange focus and
  action states, warm surfaces, dense controls, status treatments, and data-oriented layout.

## 0.2.0 — 2026-08-26

### Added

- A capability-aware API model-alias field for names used by coding agents and other
  OpenAI-compatible clients.
- Windows CI for frontend and Rust verification on every pull request and push to `main`.
- A version-consistency check covering npm, Cargo, Tauri, lockfiles, and release tags.
- Focused tests for the typed IPC boundary and live server event bridge.
- An end-to-end managed-serving test covering runtime discovery, GGUF scanning, persisted profiles,
  command generation, health checks, telemetry, logs, and process shutdown.
- Component coverage for lazy page navigation, dashboard launch guidance, model onboarding and
  rescanning, live-log filtering and snapshots, profile option storage, environment editing, and
  generated command previews.
- Contributor, security, and release-process documentation.
- Weekly dependency update checks for npm, Cargo, and GitHub Actions.

### Changed

- Unified user-facing and package branding under the LlamaPilot name.
- Restricted frontend coverage collection to maintained application source instead of generated
  Tauri build output.
- Loaded feature pages on demand and split option controls, specialised Memory/GPU and Speculative
  panels, and preview panels out of the main profile editor module.

## 0.1.4 — 2026-08-26

### Added

- Drafter-model discovery and compatibility filtering.
- MTP and external draft-model profile support.
- A capability-aware single-user 131K throughput preset.
- Searchable server logs with level and stream filters, raw mode, pause, and follow controls.

## 0.1.1 — 2026-08-26

### Changed

- Refined the initial Windows release after real installer validation.

## 0.1.0 — 2026-08-26

### Added

- Initial Windows release covering source management, builds, immutable runtimes, capability
  discovery, GGUF models, profiles, server supervision, telemetry, and logs.
