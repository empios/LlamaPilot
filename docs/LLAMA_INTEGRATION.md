# llama.cpp Integration

This document records how LlamaPilot interacts with upstream `llama.cpp`, and which upstream
facts were verified rather than assumed. Verified against `ggml-org/llama.cpp` `master`
(checked 2026-08-25): `README.md`, `docs/build.md`, `docs/speculative.md`, `docs/multi-gpu.md`,
`tools/server/README.md`, `tools/mtmd/README.md`.

The governing rule: **the selected `llama-server.exe --help` is the source of truth**, not this
document, not the app's option registry, and not a version number.

## 1. Source management

A source is a Git working copy of llama.cpp (official or a fork).

```ts
interface LlamaSource {
  id: string;
  name: string;
  repository: string;   // remote URL
  directory: string;    // absolute path to the working copy
  remote: string;       // primary remote name, normally "origin"
  currentRef: string;   // branch name, tag, or "(detached)"
  currentCommit: string;
}
```

Default repository: `https://github.com/ggml-org/llama.cpp`. It is editable so forks work.

### Default branch discovery

The default branch is discovered, never assumed to be `master`:

- Before cloning: `git ls-remote --symref <url> HEAD` → `ref: refs/heads/<branch> HEAD`.
- After cloning: `git symbolic-ref --short refs/remotes/<remote>/HEAD` → `<remote>/<branch>`.

### Status

Status uses the stable machine-readable format:

```
git status --porcelain=v2 --branch --untracked-files=normal
```

which yields `# branch.oid`, `# branch.head`, `# branch.upstream`, `# branch.ab +A -B` header
lines plus one line per changed path. From this we derive: current branch or detached HEAD,
commit SHA, upstream branch, ahead/behind counts, and whether the worktree is dirty. Commit
metadata (short SHA, subject, author date) comes from `git log -1 --format=%H%n%h%n%cI%n%s`.

### Update

Fetching and updating are separate, explicit UI operations:

```
git fetch --all --tags --prune
git merge --ff-only <upstream>
```

`Fetch` refreshes remote-tracking refs; `Update` fast-forwards from the locally known upstream.
Update is refused when tracked files are dirty, when HEAD is detached, or when the branch has no
upstream, with a specific error code for each case. Untracked scratch/build files are reported but
do not block safe Git operations. The app never runs `git reset --hard`, never force-checks-out,
and never stashes unless the user explicitly asks for it.

### Ref switching

- Local branch → `git switch <branch>`
- Remote-only branch → `git switch --track <remote>/<branch>`
- Tag or commit SHA → `git switch --detach <ref>` (detached HEAD is intentional and labelled in
  the UI)

Refs are enumerated with `git for-each-ref --format=…` over `refs/heads`, `refs/remotes`, and
`refs/tags`, so branches, remote branches, and tags are listed without network access. Arbitrary
refs can be typed directly and are validated with `git rev-parse --verify <ref>^{commit}`.

### Remotes

`git remote -v` is parsed into named fetch/push URLs. Users can add, remove, and fetch remotes
individually, which is what makes experimental forks usable without any application change.

## 2. Builds

Current upstream build shape for Windows + CUDA (`docs/build.md`):

```
cmake -S <source> -B <build-dir> -DGGML_CUDA=ON
cmake --build <build-dir> --config Release --parallel --target llama-server
```

Notes taken from upstream:

- `--config Release` matters for multi-config generators (Visual Studio); single-config
  generators (Ninja) use `-DCMAKE_BUILD_TYPE`. The build profile carries the generator so the
  right form is emitted.
- `GGML_NATIVE=OFF` produces a binary that runs on all CUDA GPUs; the default native build is
  tuned to the current machine. Exposed as a profile toggle, not silently applied.
- `CMAKE_CUDA_ARCHITECTURES` overrides compute capability detection when `nvcc` cannot detect
  the GPU.
- On Windows the required toolchain is Visual Studio 2022 with the "Desktop development with
  C++" workload.

Backends are modelled as data (`cuda`, `cpu`, later `vulkan`/`hip`/`sycl`), each mapping to a set
of CMake definitions, so adding a backend does not require new build code.

### Build directories

Build trees are keyed by `source · backend · architecture · generator · configuration` because
CMake generators cannot share a binary directory. Switching a source's ref marks its existing
configuration as potentially stale, which surfaces `Reconfigure` / `Rebuild` / `Clean build`
rather than silently deleting caches.

## 3. Immutable runtimes

The application never depends on `llama.cpp/build/bin/llama-server.exe`. After a successful
build, the complete runtime dependency set is copied into app-data:

```
runtimes/<commit-prefix>/<backend>/<runtime-id>/
  llama-server.exe
  ggml*.dll, llama.dll, and the other produced runtime libraries
  metadata.json
  capabilities/<inspection-id>/
    manifest.json
    version.stdout.txt / version.stderr.txt
    help.stdout.txt / help.stderr.txt
    devices.stdout.txt / devices.stderr.txt
```

```json
{
  "id": "<runtime id>",
  "sourceId": "<source id>",
  "sourceName": "llama.cpp",
  "repository": "https://github.com/ggml-org/llama.cpp",
  "commit": "…",
  "shortCommit": "…",
  "branch": "master",
  "backend": "cuda",
  "configuration": "release",
  "generator": "Visual Studio 18 2026",
  "buildDate": "…",
  "directory": "…",
  "executable": "…",
  "sizeBytes": 123456,
  "fileCount": 7,
  "capabilities": {
    "inspectionId": "…",
    "inspectedAt": "…",
    "version": "b7900-790b5713",
    "commit": "790b5713",
    "optionCount": 248,
    "knownOptionCount": 22,
    "deviceCount": 1,
    "speculativeTypeCount": 11
  }
}
```

Two properties follow from this:

- Updating or rebuilding llama.cpp cannot destroy a working runtime. `master @ abc123`, a second
  build of `master @ abc123`, `master @ def456`, and `experimental @ 987xyz` coexist as separate
  history entries.
- Windows file locking on a running executable never blocks a build, because a new build writes
  to a new commit-keyed directory instead of overwriting a running `llama-server.exe`.

Artifacts, initial capability discovery, raw output, and `metadata.json` are assembled under a
private staging name. Only then is the complete directory published and recorded in
`builds/metadata.json`. A crash between publication and registry persistence is repaired from the
snapshot metadata on the next start. If compilation or discovery fails, previous runtimes remain
untouched and the incomplete staging tree is removed.

## 4. Capability discovery

Whenever a runtime is created or a binary is registered, the app runs:

```
llama-server.exe --version
llama-server.exe --help
llama-server.exe --list-devices
```

and stores the raw output alongside a parsed manifest:

```ts
interface LlamaCapabilities {
  schemaVersion: number;
  version: string;
  commit?: string;
  options: Record<string, LlamaOption>;
  speculativeTypes: string[];
  devices: LlamaDevice[];
}
```

Stdout and stderr are retained separately for all three invocations. This matters because current
upstream writes version information to stderr, help and the device table to stdout, and backend
diagnostics may use stderr. Parsing uses their combined content, but the UI's Raw output tab shows
the original streams without rewriting them.

Each inspection gets a fresh UUID directory. Re-inspection writes the complete new sidecar first,
then atomically updates the runtime's `metadata.json`, and finally updates the central registry.
If the final registry write is interrupted, startup reconciliation adopts the newer snapshot
metadata. The executable and runtime libraries are never modified. Phase-3 runtimes without a
capability pointer remain readable and show an explicit **Inspect** action.

Two layers drive the UI:

**Layer 1 — known option registry.** An internal schema describes important concepts (context,
batch/ubatch, GPU layers, device, split mode, tensor split, main GPU, flash attention, KV cache
types/offload/unified buffer, fit, parallel, jinja, reasoning, speculative decoding, draft
model/device/layers/cache, and the strategy-specific draft/n-gram controls). This supplies good
labels, descriptions, and specialized controls.

**Layer 2 — runtime detection.** A registry entry is only rendered as a real control if the
selected binary's `--help` actually advertises that flag. Unsupported options are never emitted
and are hidden or shown disabled with "not supported by selected runtime".

**Unknown options.** Flags found in `--help` but absent from the registry are parsed into a
generic Advanced section. On top of that there is always an `Additional llama.cpp arguments`
escape hatch, appended verbatim as arguments (never as a shell string), so a brand-new upstream
flag is usable the day it lands.

`--spec-type` values are read from the binary rather than hardcoded. As of the checked revision
upstream advertises `none, draft-simple, draft-eagle3, draft-mtp, draft-dflash, draft-dspark,
ngram-simple, ngram-map-k, ngram-map-k4v, ngram-mod, ngram-cache`, and it accepts a
comma-separated list. This list is treated as sample data for tests only.

Speculative decoding flags differ per strategy — for example `--spec-draft-n-max` is clamped to
the trained block size for DFlash/DSpark, `ngram-mod` uses `--spec-ngram-mod-n-match/-n-min/
-n-max`, and `ngram-map-k4v` uses its own size/min-hits flags — so the UI renders per-strategy
controls instead of one shared form.

## 5. Models

Model directories are scanned recursively for `.gguf`. Only the GGUF header and key-value
metadata block are read; the tensor payload is never loaded or mmapped, so listing a directory of
multi-hundred-gigabyte models stays fast. Parsed metadata is cached by `path + size + mtime`.

The implemented reader accepts GGUF v2/v3, validates every length before advancing, captures the
small scalar fields used by the catalog, and seeks over tokenizer arrays instead of allocating
them. It returns immediately after the last key/value entry: tensor descriptors and aligned tensor
data are outside the inspection boundary. A bad file becomes a per-path scan issue and does not
hide the rest of the catalog. Cache entries live in `cache/model-metadata.json`; changing either
file size or nanosecond mtime forces a fresh parse.

Shards named `model-00001-of-00004.gguf` … are grouped into one logical model, displayed with a
shard count and total size. Only the first shard is passed to `-m`, which is what llama.cpp
expects.

Grouping uses the canonical five-digit `-00001-of-00004` suffix and cross-checks `split.no` and
`split.count` when the metadata contains them. Missing or contradictory shards remain visible as
an incomplete logical model, but only an existing shard 1 is exposed as `primaryPath` for later
command generation.

Draft/assistant GGUF files are catalogued with a separate `drafter` role instead of being offered
as primary models. Architecture metadata ending in `-assistant`/`_assistant` is authoritative;
well-known MTP, EAGLE3, DFlash, and DSpark filename markers keep older artifacts discoverable. The
profile editor prefers drafters whose base architecture matches the selected primary model (for
example `gemma4` with `gemma4-assistant`).

Multimodal projectors (`mmproj`, see `tools/mtmd/README.md`) are detected and paired with their
model when the pairing is unambiguous; the pairing is always overridable and never applied to an
obviously incompatible model. The corresponding server flags are `-mm/--mmproj`,
`-mmdev/--mmproj-device`, and `--mmproj-offload`.

Current `general.type = mmproj` files are recognised directly; filename and `clip.*` markers keep
legacy projectors discoverable. Automatic pairing is limited to compatible identities in the same
directory and happens only for exactly one candidate. Conflicting strong `general.name` or
`general.basename` identities veto a filename-only match. The Models page can switch every model
back to automatic selection, explicitly disable a projector, choose any discovered projector, or
browse to a GGUF outside the scan roots. Those user choices are persisted separately from the
disposable cache in `models/metadata.json`.

## 6. Command generation

A launch profile is a human-readable `profiles/<uuid>.json` document. It stores its runtime id and
label, model catalog id, the exact first-shard path, the exact selected `mmproj` path (when any),
host/port policy, option overrides, additional arguments, and a per-process environment map.
Runtime snapshots are immutable; model and projector paths remain pinned while editing unrelated
settings and are resolved again only when the profile's model selection changes.

Settings are three-state where llama.cpp has its own default:

| State | Behaviour |
| --- | --- |
| Default | the argument is not passed at all |
| Auto | pass the llama.cpp `auto` value, only if the binary supports it |
| Custom | pass the explicit value |

This prevents the common GUI failure where opening the app silently overrides upstream defaults.

The editor is driven by the selected runtime's persisted capability manifest. A known concept uses
its stable key when exactly one advertised flag implements it; grouped concepts and unknown flags
use the runtime's canonical flag. Switches offer Default/Pass flag and expose Disabled only when
the runtime advertises an explicit `--no-*` spelling. `Auto` is offered and accepted only when the
option's runtime-provided value hint contains `auto`. Changing runtimes clears the draft option
overrides so settings from one command surface cannot silently leak into another.

Command generation happens in Rust and emits a program plus a discrete argument array in this
order: model, host, port, optional pinned mmproj, capability-checked profile options, then the
additional argument escape hatch. Core identity flags cannot be duplicated through the structured
option map. Unknown or removed options, incomplete models, missing pinned files, uninspected
runtimes, invalid environment names, and unsupported `auto` values are rejected before save.
Additional arguments stay one argument per line and are never reparsed as a shell string.

The generated command is always shown, and can be copied as a plain command or as PowerShell.
The preview is display-only; execution uses the argument array that produced it.

The PowerShell rendering single-quotes every program/argument and temporarily applies environment
variables inside a guarded block that restores their previous process values in `finally`. The
plain preview shows the executable and arguments while the structured preview lists the environment
separately. Internally the same values are attached to `CommandSpec.environment`, so the later
process supervisor gives them only to the spawned `llama-server` process and never mutates the
machine or user environment.

### Phase 7 advanced controls

The profile editor separates general flags from two capability-filtered specialist surfaces:

- **Memory & GPU** shows the runtime-advertised K/V cache types, KV offload and unified-buffer
  switches, SWA cache policy, detected accelerator devices, layer placement, split mode and
  proportions, main GPU, Flash Attention, and memory fitting.
- **Speculative** starts with the runtime's own `--spec-type` list. Selecting a strategy reveals
  only that strategy's controls: external draft-model placement/cache, common draft thresholds,
  or the independent `ngram-simple`, `ngram-map-k`, `ngram-map-k4v`, and `ngram-mod` fields.
  `draft-mtp` is treated as an external-drafter strategy and exposes `--spec-draft-model` just like
  the other external draft strategies.
  Selecting `none` clears the other selections, and switching strategies removes now-hidden
  incompatible overrides from the draft profile.
- **Single-user throughput** is an explicit 131K preset rather than an implicit default. It requests
  one server slot, full main/draft GPU offload, Q4 K/V cache, fixed 512/256 batch sizes, disables
  automatic fitting, and applies the supported chat, reasoning, sampling, and draft-length flags.
  Unsupported flags are skipped, and network binding is deliberately left unchanged.

Cache type choices are parsed from each binary's `allowed values:` help text. Split-mode choices,
device ids, and speculative type names likewise come from the capability manifest. A newer app
can re-apply its presentation registry to an older immutable manifest in memory, so a newly
specialized control appears without rewriting the saved inspection; the persisted flags and raw
help output remain unchanged.

Rust validates the final option set before preview or save. The implemented cross-field rules
mirror the checked upstream revision:

- tensor split is marked experimental, rejects quantized K/V cache and explicit Flash Attention
  off, and cannot use `--fit on`;
- tensor proportions must be finite, non-negative, and align with an explicit device list;
- quantized V cache rejects explicit Flash Attention off even outside tensor mode;
- cache types and split modes must be among values advertised by that runtime;
- `none` cannot be combined with another speculative type, an external draft strategy requires
  an existing local draft model, inactive strategy fields are rejected, probabilities are bounded
  to 0–1, and upstream n-gram integer ranges/minimum-versus-maximum relationships are enforced;
- DFlash/DSpark previews retain a warning that llama.cpp clamps the maximum draft length to the
  draft model's trained block size.

These checks operate on the structured argument model, including profiles that originally stored
a once-unknown exact flag. The editor migrates such a value to its stable concept key when it is
next changed.

## 7. Process lifecycle

`ServerSupervisor` (Rust) owns the only permitted child process and this state machine:

```
Stopped → Starting → Loading → Ready ⇄ Busy → Stopping → Stopped
                 ↘──────────── Crashed ────────────↙
```

Tracked: generation, PID, runtime id, profile id, model, start/stop time, selected host/port,
health result, exit code, error, raw-log path, and optional telemetry. Actions: start, stop,
restart, clear the in-memory view, and subscribe/unsubscribe to ordered events. On
Windows the child is placed in a Job Object with kill-on-close before it is exposed as active, so
a crash of the app cannot leave an orphaned `llama-server.exe` or descendant.

`Starting` means the process exists. `Ready` requires the server to say so:

- `GET /health` returns **503** with `{"error":{"code":503,"message":"Loading model",…}}` while
  the model loads and **200** `{"status":"ok"}` when it is ready. This is the documented,
  public, no-API-key endpoint and is the readiness signal.
- `GET /props` reports server properties, `GET /slots` reports per-slot state (enabled by
  default, `--no-slots` disables it), `GET /metrics` is Prometheus-formatted and requires
  `--metrics`. Each is used only when enabled, and a `501 not_supported_error` response is
  treated as "endpoint disabled", not as a failure.
- `/tools` is documented upstream as internal to the Web UI and subject to change, so it is not
  used.

Port conflicts are detected before start. A fixed policy returns a dedicated `portInUse` error;
an automatic policy asks the OS for a free port and passes that resolved number in the same
structured command array used for launch. Profiles continue to remember their preferred port.

One ordered Tauri channel carries state snapshots and log entries to a persistent frontend
bridge. Subscriber ids make mount/unmount safe under React Strict Mode, while snapshot commands
repair state after a view reconnects. The Dashboard and Profiles pages use the same supervisor
state, so controls cannot start a second profile or delete an active profile/runtime.

## 8. Logs

stdout and stderr are read concurrently and split on both newline and carriage return, so a full
or progress-style pipe can never block the child. Lines are timestamped, stream/level-tagged where
the level is unambiguous, and kept in a 10,000-entry ring buffer with an explicit dropped counter.
Each run also creates a raw on-disk transcript under `logs/servers`; its line text is not decorated
or rewritten.

Startup facts worth surfacing (model load progress, GPU offload, KV allocation, listening state,
and tokens/sec) are parsed opportunistically into side metadata. The Logs page offers text search,
level and stream filters, pause/resume, auto-scroll, copy, clear-view, fact badges, and a raw mode
that shows only the untouched original line. Clearing affects the bounded view, not the transcript.
