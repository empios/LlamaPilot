# Platform support

## Supported distribution targets

| OS | Architecture | App packages | Runtime backends |
| --- | --- | --- | --- |
| Windows 10 / 11 | x64 | NSIS, MSI | CPU, CUDA |
| macOS 13 or later | Apple Silicon (arm64) | DMG | CPU, Metal |
| macOS 13 or later | Intel (x64) | DMG | CPU |
| Ubuntu 22.04 / 24.04 | x64 | AppImage, Debian | CPU, CUDA |

Each target has native test and package-build CI. The macOS deployment target is 13.0; current
CI uses macOS 15 runners, so the minimum OS must also be checked during release qualification.
Linux packages are built on Ubuntu 22.04 to avoid raising the glibc baseline. Other Linux
distributions are best effort. Linux ARM64, RPM/Flatpak, Vulkan, ROCm, and Intel GPU backends
are outside the current supported matrix. CPU remains available without a GPU.

## Installation and prerequisites

App installers contain LlamaPilot, not a model or a prebuilt llama.cpp runtime. Building a runtime
requires Git, CMake, and a C++ compiler. See [CONTRIBUTING.md](../CONTRIBUTING.md) for setup.
Metal uses Apple development tools. CUDA requires a compatible NVIDIA toolkit, host compiler,
and driver. CPU/Metal builds use static llama.cpp libraries by default; Metal shader sources are
embedded. Explicit custom CMake arguments can change dependency requirements.

- macOS: choose the DMG matching your architecture, open it, and drag the app to Applications.
- Linux AppImage: `chmod +x <file>.AppImage`, then launch. FUSE 2 or AppImage extraction may be
  required by the distribution. A working graphical session is required.
- Debian package: `sudo apt install ./<file>.deb` resolves runtime package dependencies.
- Windows: NSIS is the interactive installer; MSI supports managed deployments.

Releases include SHA256SUMS.txt. Compare the digest against the asset from the same GitHub release.
Without distributor signing credentials, Windows packages are unsigned and macOS packages use
ad-hoc signing without notarization. For an unnotarized Mac download from a trusted release,
use System Settings → Privacy & Security → Open Anyway if macOS blocks it. Do not disable
Gatekeeper globally. Release notes must say whether signing/notarization was performed.

## Behavior and limitations

Tool lookup preserves PATH order and appends standard system, Homebrew, and CUDA locations.
Configured executable paths take precedence. Shell startup scripts are never executed by the app.
Apple GPU memory is shared with the CPU; dashboard memory figures describe system memory, not
an independent VRAM pool. Performance plans use memory budgets reported by the selected Metal
runtime and never substitute the whole system RAM as dedicated GPU memory.

Unix process groups are supervised by pipe watchdogs: cancellation, ordinary shutdown, and
unexpected parent termination kill adopted groups. Deliberately daemonized descendants that
leave the group are outside this guarantee. There is a small spawn-to-adoption interval; this is
not a kernel Job Object equivalent. Windows retains Job Object supervision.

Application data follows Tauri's OS-specific app-data location under the existing
`com.llamacontrol.app` identifier. User workspace paths and existing Windows data remain compatible.
Model files and source checkouts are user-owned and must survive uninstall.

## Release qualification

Before announcing a platform release, record checks against the actual packaged app:
install/desktop launch, tool detection, clone, CPU build, supported GPU build, snapshot relocation,
GGUF selection/download, capability inspection, serving a request, stop/restart, cancellation,
app exit with active children, settings persistence, and uninstall. Linux also needs X11/Wayland
checks. GPU inference must be tested on real corresponding hardware.

See [CROSS_PLATFORM_PLAN.md](CROSS_PLATFORM_PLAN.md) for this implementation's local validation
record. A build passing in CI does not by itself prove every hardware or desktop combination.
