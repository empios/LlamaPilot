# macOS and Linux delivery plan

## Scope and acceptance

Ship native Apple Silicon and Intel macOS DMGs, Linux x64 AppImage and Debian packages,
and preserve Windows NSIS/MSI releases. Support CPU everywhere, Metal on Apple Silicon,
and CUDA on Windows/Linux with the required toolkit. The immediate local deliverable is
an Apple Silicon DMG. Signing/notarization requires distributor credentials; locally built
ad-hoc signed packages must be identified accurately. Other OS qualification must be
reported separately from checks run on this Mac.

## Implementation sequence

1. **Platform/process foundation:** supervise Unix descendants, terminate on cancellation and
   normal shutdown, test cleanup, document abnormal-exit limitations; resolve tools and child
   PATH for desktop launches without modifying the user's environment.
2. **Build/runtime support:** detect native C++ tools, offer appropriate CPU/CUDA/Metal choices,
   synchronize Rust/TypeScript schemas, explicitly select backends, preserve executable modes
   and relocate runtime libraries/resources, and test snapshots after source removal.
3. **Platform-aware UI:** platform defaults and diagnostics, POSIX command previews and agent
   snippets, Apple hardware/unified-memory reporting, capability-aware performance behavior,
   platform-neutral examples and help.
4. **Packaging and CI:** OS-specific Tauri bundle configurations, native verification matrix,
   portable version verification, explicit tag selection for manual releases, one draft release
   collecting all packages and checksums before publication, optional protected Apple signing.
5. **Official project documentation:** platform/version/backend support matrix, installation and
   development prerequisites, release qualification checklist, issue fields, architecture and
   changelog updates. Distinguish supported targets from validation actually completed.
6. **Validate and deliver:** frontend tests/typecheck/build; Rust formatting/Clippy/tests; native
   macOS package build; inspect/mount DMG and validate app architecture/signature/launch; test a
   real llama.cpp CPU/Metal runtime where local tooling permits. Save DMG and checksum in
   `artifacts/`, record results here, and provide a clickable download.

## Release qualification

For each platform: install, launch from desktop, detect tools, clone source, build server,
snapshot/inspect, select/download GGUF, serve a request, stop/restart, cancel a build, exit with
active children, reopen with settings intact, uninstall without deleting user models/sources.
Validate CUDA/Metal on actual hardware and Linux under X11/Wayland before claiming those
checks passed. A complete release must contain every required architecture and format.

## Execution record (updated during delivery)

- Planning: repository audited; Apple Silicon Mac with Node 22 and Apple command-line tools.
- Local prerequisites: Rust is absent; install an isolated temporary toolchain for the build.
- Signing: no Developer ID signing identity is installed; local DMG will be ad-hoc signed.

- Implemented: Unix groups/watchdogs; platform toolchains and Metal; portable snapshots and
  shell snippets; Apple hardware reporting; host packages; CI/release matrix; support docs.
- Version: 0.3.0, preserving the published 0.2.0 release.
- Local validation: 76 frontend tests pass with coverage; 200 Rust unit tests and 15 integration
  tests pass; the ignored watchdog fixture runs through the crash test. Clippy passes.
- Real runtime: upstream llama.cpp Metal server compiled successfully using Apple command-line
  tools and embedded shaders. Further relocation/inference checks are recorded below.
- Local DMG: build in progress. CI/platform and signing qualification remain distinct from these
  Apple Silicon checks.

- Real Metal inference: a relocated, statically linked server completed two model-load / HTTP
  completion / stop cycles through LlamaPilot's ServerSupervisor on Apple M4 Max. The locally
  available Bielik 11B model generated 8 tokens per request. System frameworks are the only
  dynamic dependencies after requesting static OpenSSL and invalidating its old CMake cache.
- UI launch: the packaged app displays version 0.3.0, Apple GPU status, native C++/Make tools,
  and CPU/Metal choices without requiring MSVC or CUDA.
- First remote CI run: macOS ARM64, macOS Intel, and Ubuntu package/test jobs passed. Windows
  found a test-module ordering lint, now fixed. The final revision is being verified separately.
- DMG verification caught incomplete default signing; ad-hoc bundle signing is now explicit in
  the macOS configuration and both CI paths verify the mounted app signature.

- Final local checks: 202 Rust unit tests and 15 integration tests pass, 76 frontend tests pass,
  formatting/Clippy/typecheck/build pass. The signed DMG passes hdiutil verification and strict
  codesign validation, reports arm64/version 0.3.0, and launches with the corrected generator list.
- CPU fallback: a separate GGML_METAL=OFF build also completed two supervised model-load,
  completion, stop/restart cycles. Metal and CPU binaries link only to system frameworks/libraries.
- Release assembly: a complete six-package fixture produced verified SHA-256 entries; removing
  the Intel Mac asset was rejected. Optional signing setup handles absent credentials without
  setting empty APPLE_* variables, and rejects partial credentials before packaging.
- Local delivery: `artifacts/LlamaPilot_0.3.0_aarch64.dmg`, checksum sidecar and VALIDATION.md.
  The full native CI and tagged release publication are tracked in PR #12.
