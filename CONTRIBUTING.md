# Contributing to LlamaPilot

LlamaPilot manages the user's own llama.cpp server. Native code, tests, and installers must
remain compatible with the [supported platforms](docs/PLATFORM_SUPPORT.md).

## Development setup

Use Node.js 22+, Rust stable, and Git everywhere. Install platform build dependencies:

- Windows: MSVC Rust target and Visual Studio's **Desktop development with C++** workload.
- macOS: `xcode-select --install`; install CMake for exercising llama.cpp builds
  (`brew install cmake`; Ninja is optional).
- Ubuntu 22.04/24.04:

```sh
sudo apt-get update
sudo apt-get install build-essential curl git cmake ninja-build libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

AppImage packaging on Ubuntu 22.04 also needs `libfuse2` (`libfuse2t64` on Ubuntu 24.04).
CUDA is optional and requires a supported toolkit, host compiler, and NVIDIA driver.

```sh
npm ci
npm run tauri dev
```

## Verification

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

CI runs native tests and packaging on Windows x64, macOS Apple Silicon/Intel, and Linux x64.
Tests that bind loopback ports or spawn real descendants require ordinary OS access.
The ignored Unix watchdog fixture is invoked by its parent-death integration test automatically.

Keep process arguments as arrays. Never run user input through a shell; generated shell previews
are for copying only. Preserve the invariants in [ARCHITECTURE.md](docs/ARCHITECTURE.md).
Add focused tests for changed behavior, especially platform boundaries and visible state transitions.
Update CHANGELOG.md for user-visible, persistence, compatibility, or release-process changes.
