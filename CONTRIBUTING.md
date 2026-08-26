# Contributing to LlamaPilot

Focused bug fixes, compatibility updates, tests, and usability improvements are welcome. Keep the
application's scope narrow: LlamaPilot manages the user's own `llama.cpp` server; it is not a chat
client or inference backend.

## Development setup

Use Windows 10 or 11 with Node.js 20+, Rust stable with the MSVC target, Git, and Visual Studio
2022's **Desktop development with C++** workload. CMake and CUDA are only required for exercising
the corresponding build paths.

```powershell
npm ci
npm run tauri dev
```

Do not run Git, CMake, or user-provided values through a shell string. Process arguments must stay
as discrete values owned by the Rust backend. Preserve the safety invariants described in
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Before opening a pull request

Run the same checks as CI:

```powershell
./scripts/check-version-sync.ps1
npm run typecheck
npm run test:coverage
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Add tests for behavior changes. Parsers and validation should normally have pure unit tests;
process ownership, Git behavior, and other OS boundaries belong in integration tests. Frontend
changes should cover user-visible state transitions rather than implementation details.

Keep pull requests focused and update `CHANGELOG.md` under **Unreleased** when the change affects
users, persisted data, compatibility, or the release process.
