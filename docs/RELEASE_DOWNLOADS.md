Download the package for your platform and architecture:

- Windows x64: `*-setup.exe` (interactive) or `*.msi`.
- macOS Apple Silicon: `*_aarch64.dmg`; macOS Intel: `*_x64.dmg`. Open and drag to Applications.
- Linux x64: `*.AppImage` or `*_amd64.deb`. Mark AppImages executable before launching.

Verify downloads with `SHA256SUMS.txt`. Packages may be unsigned/ad-hoc signed unless this release
explicitly confirms Developer ID signing and Apple notarization. See the platform support guide
for Gatekeeper handling, Linux dependencies, and supported OS versions.

LlamaPilot builds your own llama.cpp runtime; Git, CMake, a native C++ compiler, and a GGUF model
are required for serving. CPU works everywhere, Metal on Apple Silicon, CUDA on Windows/Linux.
