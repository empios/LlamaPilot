# Architecture

LlamaPilot is a Windows-first desktop control panel for upstream `llama.cpp`. It manages
llama.cpp Git sources, builds `llama-server` binaries, snapshots them into immutable runtimes,
discovers what each runtime supports, and launches/monitors the server process.

It is not an inference backend and not a chat client. Every capability it exposes belongs to
the user's own `llama-server.exe`.

## Naming isolation

The product identity is concentrated in these locations so a rename stays cheap:

| Concern | Location |
| --- | --- |
| Product/window title, bundle identifier | `src-tauri/tauri.conf.json` |
| Frontend display strings | `src/lib/branding.ts` |
| Rust crate name | `src-tauri/Cargo.toml` |
| Stable user workspace folder | `src-tauri/src/config/paths.rs` |

The workspace folder deliberately does not follow product renames: changing it would strand or
silently duplicate large source and build trees already owned by the user.

## Process model

```
┌──────────────────────────────┐        typed Tauri commands        ┌──────────────────────────────┐
│  WebView (React + Vite)      │  ───────────────────────────────▶  │  Rust core (tokio)           │
│  Zustand · TanStack Query    │  ◀─────────────────────────────── │  git · cmake · llama-server  │
│  Zod-validated boundaries    │      Channels (streamed output)    │  filesystem · hardware       │
└──────────────────────────────┘                                    └──────────────────────────────┘
```

The frontend never executes shell commands. It cannot construct a process invocation. It calls
named Rust commands with structured arguments; Rust owns argument arrays, path validation, and
process spawning. The Tauri shell plugin is deliberately not installed.

Long-running operations (clone, fetch, build, server logs) stream through
[Tauri channels](https://v2.tauri.app/develop/calling-frontend/#channels) rather than the event
system, because channels preserve ordering and are optimized for throughput.

## Rust module map

```
src-tauri/src/
  lib.rs            application builder, state wiring, command registration
  error.rs          AppError / AppErrorPayload — every command returns Result<T, AppError>
  state.rs          AppState: paths, settings store, source registry
  commands/         thin Tauri command layer, one module per domain
    settings.rs     read/patch persisted settings
    system.rs       app paths, hardware snapshot, reveal-in-explorer
    sources.rs      clone / status / fetch / update / refs / remotes / switch
    server.rs       start / stop / restart, snapshots, live-log channel subscription
  config/
    paths.rs        AppPaths — resolves app-data layout from Tauri path API
    settings.rs     Settings model with serde defaults and a schema version
    store.rs        JsonStore<T>: atomic read/modify/write of a JSON document
  process/
    command.rs      CommandSpec — program + argument array + cwd + env overlay
    runner.rs       capture and line-streaming execution on tokio
  git/
    runner.rs       git invocation with git-specific error mapping
    status.rs       `git status --porcelain=v2 --branch` parser
    refs.rs         branch/tag/commit parsers, ref classification
    remote.rs       `git remote -v` parser
    repository.rs   high-level safe operations (clone, fetch, ff-only update, switch)
  sources/
    record.rs       LlamaSource persisted record
    registry.rs     sources/metadata.json registry
    service.rs      orchestration: registry + git + safety rules
  hardware/         CPU, RAM, NVIDIA GPU snapshot
  logging/          tracing subscriber, rolling file log
  platform/         Windows specifics (no console window, reveal in explorer, job objects)
  build/            (Phase 3) CMake toolchain detection, build execution, cancellation ownership
  runtime/          transactional snapshots, recovery metadata, capability re-inspection
  llama/            discovery runner, pure parsers, known-option registry, versioned sidecars
  gguf/             bounded GGUF v2/v3 metadata reader and split-name parser
  models/           recursive catalog, HF downloader, fingerprint cache, shard grouping, mmproj choices
  profiles/         one-file profiles, capability-checked generation, advanced dependency validation
  performance/      all-GPU candidates, sweep supervision/ranking, benchmark parsing and history
  server/           one-child supervisor, lifecycle state, health probes, bounded/raw logs
```

Rule: parsing is pure and unit-testable; it never spawns a process. Process execution lives in
`process/` and `git/runner.rs`. That split is why `git status`, ref, and `--help` parsing can be
tested from fixtures with no llama.cpp installed.

## Frontend module map

```
src/
  app/          shell, providers, navigation definition
  components/   shared presentation (ui/ is shadcn, rest is app-level)
  features/     dashboard · models · profiles · performance · runtime · builds · logs · settings
  hooks/        cross-feature hooks
  lib/          tauri bridge, formatting, branding, cn()
  stores/       Zustand client state (navigation, theme, transient UI)
  types/        shared domain types mirroring the Rust payloads
```

`src/lib/ipc.ts` is the only module that imports `@tauri-apps/api/core`. Every command is
declared once there with its argument and result types, so a Rust signature change surfaces as
a TypeScript error at a single location.

State ownership:

- **TanStack Query** owns anything the Rust side is the source of truth for (settings, sources,
  Git status, hardware, runtimes, models, profiles, server state, and server logs). Server channel
  events update the corresponding query cache in order; other mutations invalidate query keys.
- **Zustand** owns client-only state (current page, theme, panel sizes, draft forms).
- **Zod** validates at the IPC boundary for payloads that come from parsing external output, so
  malformed data fails loudly instead of rendering as `undefined`.

## Error model

Every command returns `Result<T, AppError>`. `AppError` serializes to:

```json
{
  "code": "dirtyWorktree",
  "message": "The repository has uncommitted changes.",
  "hint": "Commit, stash, or discard the changes before switching refs.",
  "details": "<raw git stderr>"
}
```

`code` is a closed enum, so the UI can branch on failure kind without string matching. `message`
is the actionable sentence. `details` always carries the untouched raw output — the UI shows it
behind an expander and never discards it.

## Persistence

Plain JSON files under the Tauri app-data directory, written atomically (temp file + rename) so
a crash mid-write cannot corrupt state. Tauri resolves that directory from the bundle
identifier, so on Windows it is `%APPDATA%\com.llamacontrol.app`. This legacy identifier remains
stable across the LlamaPilot rename so existing installations keep their settings and runtime data:

```
%APPDATA%/<bundle identifier>/
  settings.json
  sources/metadata.json
  builds/metadata.json                 successful-build/runtime registry
  models/metadata.json                 persistent per-model mmproj overrides
  profiles/*.json
  performance/history.json             newest-first bounded benchmark history
  runtimes/<commit>/<backend>/<runtime-id>/
    llama-server.exe
    runtime libraries
    metadata.json                      self-describing recovery record
    capabilities/<inspection-id>/
      manifest.json                    parsed runtime-specific capabilities
      version.stdout.txt / .stderr.txt
      help.stdout.txt / .stderr.txt
      devices.stdout.txt / .stderr.txt
  cache/model-metadata.json
  logs/
    servers/llama-server-<timestamp>-<generation>-<profile>.log
```

Git working copies and build trees are large and live wherever the user chooses (default
`%USERPROFILE%/LlamaControl/sources`), configured in settings — never inside app-data.

## Safety invariants

1. No shell string is ever constructed for execution; only argument arrays.
2. Git operations that could destroy work (`reset --hard`, force checkout, implicit stash) are
   not implemented. Updates are fast-forward-only and refuse to run on a dirty worktree.
3. A successful runtime's executable and libraries are immutable. Files, initial capability
   inspection, and metadata are assembled under a private staging name and published with one
   directory rename. Rebuilding never overwrites an existing runtime; repeated builds get
   distinct ids. A re-inspection adds a new immutable sidecar and then atomically moves the
   metadata pointer. Startup reconciliation can reconstruct or refresh the central registry after
   an interrupted write.
4. Child processes are spawned with `CREATE_NO_WINDOW` on Windows so no console flashes.
5. Environment overlays apply to the spawned child only; the system environment is never mutated.
6. Capability manifests stay immutable. Loading may enrich their presentation metadata with the
   current known-option registry, but support still comes exclusively from flags and values saved
   from that runtime's own help output.
7. One supervisor owns at most one server child. The child is adopted into a Windows Job Object
   before it is exposed as active; stop, app exit, and supervisor drop terminate the complete
   process tree. An active profile or runtime cannot be deleted.
8. Server output is drained concurrently from stdout and stderr. Memory is capped at 10,000
   entries with an explicit dropped count; the per-run file receives the untouched line text.
   Timestamps, levels, streams, and parsed startup facts are side metadata, never rewrites.
9. A manual Performance Lab benchmark never starts, stops, or mutates the active process implicitly.
   The explicit automatic sweep requires an idle supervisor, holds a single-sweep permit, derives
   every candidate from the original profile, and owns every start/stop until it finishes. It saves
   only the fastest successful candidate; cancellation or total failure restores the original
   profile. Manual profile mutations and server starts are rejected while the permit is held.
   Requests disable prompt-cache reuse, and history captures the candidate, sweep, placement, and
   GPU snapshot that produced each result.

## Testing

Rust unit tests live beside their modules and run without Git, CMake, or llama.cpp present.
Capability parser tests use captured upstream-shaped fixtures, including wrapped help text,
unknown future flags, dynamic speculative types, and device memory. Profile tests cover tensor/KV
constraints, device proportions, speculative strategy dependencies, runtime-advertised cache
values, port selection, health/metric parsing, bounded logs, and the server lifecycle. Integration
tests exercise real Git repositories and Windows Job Objects, including detached grandchildren
and the cancel-before-process-adoption race. The managed-serving fixture also executes the coding
benchmark over HTTP and verifies current llama-server timing fields. Performance tests cover clean
candidate application, generation-first ranking, cancellation, and exclusive sweep ownership.
Frontend tests cover state derivation, retained build output, capability and server payloads,
candidate application, and per-strategy control selection.
