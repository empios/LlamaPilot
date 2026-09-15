<p align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="LlamaPilot app icon" />
</p>

<h1 align="center">LlamaPilot</h1>

<p align="center">
  Build, version, inspect, and run your own <code>llama.cpp</code> servers from one native app for Windows, macOS, and Linux.
</p>

<p align="center">
  <img alt="Windows" src="https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows" />
  <img alt="macOS" src="https://img.shields.io/badge/macOS-13%2B-black?logo=apple" />
  <img alt="Linux" src="https://img.shields.io/badge/Linux-Ubuntu%2022.04%20%7C%2024.04-E95420?logo=ubuntu" />
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?logo=rust" />
  <img alt="React" src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=111827" />
  <img alt="License" src="https://img.shields.io/badge/license-MIT-green" />
</p>

LlamaPilot is a cross-platform desktop control panel for upstream
[llama.cpp](https://github.com/ggml-org/llama.cpp). It manages the whole local serving workflow
without hiding the tools underneath: clone a source, build `llama-server`, keep immutable runtime
snapshots, discover what each binary supports, create launch profiles, and monitor the running
server.

Your own `llama-server` (`llama-server.exe` on Windows) runs inference, built from the source and
revision you choose. Use CPU on every supported platform, Metal on Apple Silicon, or NVIDIA
CUDA on Windows and Linux.

**[Download LlamaPilot](https://github.com/empios/LlamaPilot/releases/latest)** ·
[Website](https://empios.github.io/LlamaPilot/) ·
[Platform support](docs/PLATFORM_SUPPORT.md) · [Build from source](#build-from-source)

<p align="center">
  <a href="docs/images/dashboard.png">
    <img src="docs/images/dashboard.png" alt="LlamaPilot dashboard showing llama.cpp source, runtime, model, and hardware status" />
  </a>
</p>

## Screenshots

| Build and version llama.cpp | Create precise launch profiles |
| --- | --- |
| [![LlamaPilot build screen](docs/images/build.png)](docs/images/build.png) | [![LlamaPilot profiles screen](docs/images/profiles.png)](docs/images/profiles.png) |

## Why LlamaPilot?

| Without it | With LlamaPilot |
| --- | --- |
| Long command lines copied between text files | Named, persistent launch profiles with exact previews |
| Rebuilding over the last working binary | Immutable runtime snapshots that safely coexist |
| Guessing whether a flag exists in your build | Controls generated from that binary's own `--help` output |
| Manual Git, CMake, CUDA, and model bookkeeping | One guided desktop workflow with actionable errors |
| A terminal full of mixed startup output | Searchable live logs, parsed facts, and untouched raw output |
| Orphaned compilers or servers after cancellation | Platform supervisors terminate owned server and compiler groups |

## Highlights

- **Own your runtime** — clone upstream llama.cpp or a fork, fetch remotes, switch branches, tags,
  and commits, and update with fast-forward-only safety.
- **Build without a developer prompt** — detect native C++ tools, CMake, Ninja, and CUDA; build CPU,
  CUDA, or Apple Metal profiles; retain live output; cancel the entire build tree safely.
- **Never lose a working build** — every successful build becomes a self-describing, immutable
  runtime containing the executable, required libraries, metadata, and capability manifest.
- **Use what the binary actually supports** — runtime controls come from `--version`, `--help`,
  and `--list-devices`, including unknown flags introduced by newer llama.cpp versions.
- **Understand large model folders quickly** — scan GGUF metadata without reading tensor payloads,
  group split models, cache results, and pair multimodal projectors conservatively.
- **Download the exact GGUF you choose** — browse a public Hugging Face model repository,
  select one file or a complete shard set, and save it into an already configured model folder
  with progress, cancellation, and an automatic catalog refresh.
- **Tune advanced serving setups** — configure KV cache types, GPU placement, tensor splits,
  Flash Attention, and runtime-advertised speculative decoding strategies with cross-field checks.
- **Measure one model across every GPU** — generate VRAM-weighted layer, row, and supported tensor
  candidates, apply one to a profile, and compare repeatable coding-prompt timings in Performance
  Lab.
- **Name models for API clients** — assign comma-separated aliases that coding agents and other
  OpenAI-compatible clients can use in their `model` field when the runtime supports `--alias`.
- **Connect a coding agent without guessing** — start a profile, verify `/v1/models` and
  `/v1/chat/completions`, then copy the effective URL, model ID, and ready configuration for
  OpenCode, Pi, Aider, or an OpenAI JavaScript client from Agent Connect.
- **Run and observe** — Start, Stop, and Restart from the Dashboard or Profiles page; readiness
  comes from `/health`, while `/props`, `/slots`, and `/metrics` enrich optional telemetry.
- **Keep the raw truth** — stdout and stderr are drained concurrently into a bounded live view and
  a per-run raw transcript. Parsed levels and startup facts never rewrite the original line.
- **Use a coherent desktop design language** — the frontend adapts the MIT-licensed
  [Pangolin Design System](https://github.com/empios/Pangolin), with warm Paper and aubergine
  Terminal themes, compact operational controls, and locally bundled Ubuntu Sans typography.

## Quick start

### Install

Native installers are available in [LlamaPilot v0.4.2](https://github.com/empios/LlamaPilot/releases/tag/v0.4.2).
Check [the latest release](https://github.com/empios/LlamaPilot/releases/latest) for newer versions.
Choose the package matching your operating system and processor:

| Platform | Download | Backend support |
| --- | --- | --- |
| Windows 10/11 x64 | [Setup EXE](https://github.com/empios/LlamaPilot/releases/download/v0.4.2/LlamaPilot_0.4.2_x64-setup.exe) · [MSI](https://github.com/empios/LlamaPilot/releases/download/v0.4.2/LlamaPilot_0.4.2_x64_en-US.msi) | CPU, NVIDIA CUDA |
| macOS 13+ Apple Silicon | [DMG for Apple Silicon](https://github.com/empios/LlamaPilot/releases/download/v0.4.2/LlamaPilot_0.4.2_aarch64.dmg) | CPU, Metal |
| macOS 13+ Intel | [DMG for Intel](https://github.com/empios/LlamaPilot/releases/download/v0.4.2/LlamaPilot_0.4.2_x64.dmg) | CPU |
| Ubuntu 22.04/24.04 x64 | [AppImage](https://github.com/empios/LlamaPilot/releases/download/v0.4.2/LlamaPilot_0.4.2_amd64.AppImage) · [Debian package](https://github.com/empios/LlamaPilot/releases/download/v0.4.2/LlamaPilot_0.4.2_amd64.deb) | CPU, NVIDIA CUDA |

On macOS, open the DMG and drag LlamaPilot to Applications. On Linux, make the AppImage
executable before launching it, or install the Debian package with `sudo apt install ./<file>.deb`.
See [platform support](docs/PLATFORM_SUPPORT.md) for prerequisites, signing status, and limitations.

The v0.4.2 macOS apps are **ad-hoc signed, without Apple notarization**. Windows installers are
unsigned and may trigger Microsoft Defender SmartScreen. Download from this repository's releases
and compare the file's SHA-256 digest with
[SHA256SUMS.txt](https://github.com/empios/LlamaPilot/releases/download/v0.4.2/SHA256SUMS.txt).

The installer contains the LlamaPilot app. To build and serve your first runtime, install **Git,
CMake, and a native C++ compiler**, and choose a GGUF model. See the
[platform setup instructions](CONTRIBUTING.md) for the required tools; CUDA additionally needs
the NVIDIA toolkit and driver.

### Application updates

Version 0.4.1 includes **Settings → Updates**. Checks run at startup and every 24 hours while
the app is open. Background downloads and installation when idle are optional. Automatic
installation shows a 30-second restart countdown and waits for servers, builds, downloads,
benchmarks and open editors to finish. Choose **Later** to defer that version for the session.

In-app installation supports NSIS on Windows, installed macOS apps and writable Linux
AppImages. MSI, DEB and standalone executables use manual package updates. Existing v0.3.0
users must install v0.4.1 or a newer release manually to enable future in-app updates. See
[updater release setup](docs/RELEASING.md#updater-signing-and-first-release) for signing and qualification.

### First server

1. Open **Settings** and add one or more directories containing `.gguf` models.
2. Open **Runtimes** and clone upstream llama.cpp or register an existing working copy.
3. Open **Build**, choose an available CPU, CUDA, or Metal backend, and build `llama-server`.
4. Inspect the new runtime so LlamaPilot can discover its exact command surface.
5. Open **Models**, verify the model and optional multimodal projector pairing. You can also choose
   **Hugging Face**, paste a public model repository URL or ID, and download an exact GGUF
   selection directly into one of the configured model folders.
6. Create a **Profile**, review the generated command, and press **Start**.
7. Watch readiness, slots, throughput, and untouched output on **Dashboard** and **Logs**.
8. Open **Performance** and run **Auto-tune** to test every runtime-valid GPU placement. LlamaPilot
   performs isolated model loads, saves the fastest successful generation configuration, and leaves
   the server stopped for review or launch.
9. Open **Agent Connect**, start the tuned profile, run the compatibility test, and copy the
   generated connection settings into the coding agent.

## Safety by design

- The WebView cannot run arbitrary commands. It can only call named Rust operations; the Tauri
  shell plugin is intentionally absent.
- Process arguments are arrays, never shell strings. User values cannot turn into shell syntax.
- Git operations never reset, force-checkout, or implicitly stash user work. Updates are
  fast-forward-only and dirty tracked files block destructive transitions.
- Builds publish transactionally. Failed or cancelled work never replaces an existing runtime.
- Environment overrides belong only to the launched child and never modify machine or user state.
- Exactly one server is supervised at a time, and an active profile or runtime cannot be deleted.
- Windows Job Objects use kill-on-close. Unix process groups and pipe watchdogs clean up owned
  descendants after cancellation or parent exit; deliberately daemonized processes are outside that guarantee.

## Technology

- **Desktop:** Tauri 2 and Rust on Tokio
- **UI:** React 19, TypeScript, Vite, Tailwind CSS, Radix UI, and Pangolin design tokens
- **State:** TanStack Query for backend state and Zustand for local UI state
- **Boundaries:** Serde on Rust and Zod validation in TypeScript
- **Integration:** Git, CMake, upstream llama.cpp CLI discovery, GGUF v2/v3 metadata, and the
  documented llama-server HTTP endpoints

The architecture keeps process execution and filesystem access in Rust. The frontend is a typed,
reactive control surface rather than a privileged shell.

Third-party design and font attributions are listed in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Build from source

### Requirements

All platforms: Node.js 22+, Rust stable, Git, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

- **Windows:** Visual Studio C++ workload and MSVC Rust target.
- **macOS:** Apple Command Line Tools (`xcode-select --install`). Install CMake for llama.cpp builds.
- **Linux:** C++ compiler, Make or Ninja, WebKitGTK 4.1 development libraries, and packaging tools.
- **Optional:** NVIDIA CUDA Toolkit and driver for CUDA runtimes on Windows/Linux.

See [CONTRIBUTING.md](CONTRIBUTING.md) for exact setup commands.

```sh
npm ci
npm run tauri dev
```

### Verification and installers

```sh
npm run check:version
npm run typecheck
npm run test:coverage
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

The final command builds the host platform's packages under `src-tauri/target/release/bundle/`.
Version tags run the desktop release workflow. All required packages must succeed before one
complete GitHub Release is published. See [the release process](docs/RELEASING.md).

## Project status

**v0.4.2 is released for Windows, macOS, and Linux**, with signed application-update packages
and refreshed Paper/Terminal themes. CI tests and packages Windows x64,
macOS Apple Silicon, macOS Intel, and Linux x64. CPU and Metal serving were also exercised on
an Apple M4 Max. See [platform support](docs/PLATFORM_SUPPORT.md) and the
[release validation notes](https://github.com/empios/LlamaPilot/releases/tag/v0.4.2) for the
supported targets and remaining hardware and desktop qualification.

Version 0.4.2 fixes model downloads appearing stuck at 0% and allows profiles to use MTP
heads embedded in the main GGUF without a separate draft file. With a runtime that advertises
`draft-mtp`, choose **Profiles → Speculative → Use MTP from main GGUF** when your model includes
those heads. Separate draft files remain supported. See [the changelog](CHANGELOG.md).

The initial eight-phase implementation roadmap is complete:

1. Native shell, settings, typed IPC, and structured errors
2. Safe llama.cpp source and remote management
3. Toolchain detection, builds, cancellation, and immutable runtimes
4. Runtime-specific capability discovery
5. GGUF catalog, split grouping, and multimodal projector pairing
6. Persistent launch profiles and exact command previews
7. Advanced memory, multi-GPU, and speculative decoding controls
8. Server supervision, health telemetry, and live/raw logs

The current optimization track is focused on one coding-model server using all available GPUs.
Performance Lab is the measurement foundation: it creates capability-aware placement candidates,
records prompt and generation throughput, and keeps results tied to the exact profile, runtime,
model, and GPU snapshot. Its automatic sweep tests each candidate under the same deterministic
request, skips unsupported placements, and applies the fastest successful generation result without
weakening the single-server safety model.

The current version is an early release focused on a trustworthy local llama.cpp workflow.
Bug reports and focused pull requests are welcome.

## Documentation

- [Platform support](docs/PLATFORM_SUPPORT.md) — operating systems, architectures, backends,
  installation, signing, and known limitations
- [Architecture](docs/ARCHITECTURE.md) — modules, state ownership, persistence, errors, and safety
  invariants
- [llama.cpp integration](docs/LLAMA_INTEGRATION.md) — verified upstream behavior behind sources,
  builds, GGUF handling, command generation, health checks, and logs
- [Release process](docs/RELEASING.md) — versioning, verification, tagging, and installer publishing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and [SECURITY.md](SECURITY.md)
for responsible vulnerability reporting.

## License

[MIT](LICENSE) — use it, study it, adapt it, and contribute improvements back if you can.
