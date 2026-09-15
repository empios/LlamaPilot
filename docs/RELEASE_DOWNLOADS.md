LlamaPilot 0.4.2 fixes model download progress and supports MTP heads embedded in the main GGUF.

- Hugging Face downloads now update the progress bar and byte counter instead of staying at 0%.
- Profiles using `draft-mtp` can be saved without a separate draft file. Choose
  **Profiles → Speculative → Use MTP from main GGUF** when your model includes MTP heads.
  This clears a saved draft-file selection; separate MTP files remain supported. Strategies
  that require an external draft model still validate its file.
- README installer links, project status, and application-update instructions are refreshed.

Users of v0.4.1 can check **Settings → Updates** for this version. In-app installation supports
NSIS installations on Windows, installed macOS apps, and writable Linux AppImages;
MSI and Debian packages require a manual installer update.

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

Changes: https://github.com/empios/LlamaPilot/pull/15
