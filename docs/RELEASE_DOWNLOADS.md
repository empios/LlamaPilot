LlamaPilot 0.4.1 adds signed application updates in Settings, optional background downloads
and installation when idle, and refreshed Paper/Terminal desktop themes.

It also fixes macOS updater packaging and checks each build for missing packages and signatures
before upload. The earlier v0.4.0 publication was stopped before a release was created.

Users of the original v0.3.0 release must install this version manually. Local v0.3.0
installers built with the production updater key can discover this release through Settings.
Automatic downloading and installation remain opt-in; installation waits for active work
and open editors and shows a restart countdown.

Download the package for your platform and architecture:

- Windows x64: `*-setup.exe` (interactive) or `*.msi`.
- macOS Apple Silicon: `*_aarch64.dmg`; macOS Intel: `*_x64.dmg`. Open and drag to Applications.
- Linux x64: `*.AppImage` or `*_amd64.deb`. Mark AppImages executable before launching.

Updater packages have Minisign signatures and are listed in `latest.json`. Verify manual
downloads with `SHA256SUMS.txt`. OS code signing is separate: packages may be unsigned/ad-hoc signed unless this release
explicitly confirms Developer ID signing and Apple notarization. See the platform support guide
for Gatekeeper handling, Linux dependencies, and supported OS versions.

Validation covers automated tests, native packaging, and updater signature verification.
Installed upgrades between versions and the full Windows UAC/macOS/Linux desktop upgrade
qualification matrix have not yet been completed; see `docs/RELEASING.md`.

LlamaPilot builds your own llama.cpp runtime; Git, CMake, a native C++ compiler, and a GGUF model
are required for serving. CPU works everywhere, Metal on Apple Silicon, CUDA on Windows/Linux.
