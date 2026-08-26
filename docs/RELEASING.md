# Release process

Windows installers are built and published by `.github/workflows/release.yml` when a version tag
is pushed. Releases must come from a clean, verified commit on `main`.

## 1. Prepare the release

1. Choose the next version and update it in:
   - `package.json` and `package-lock.json`;
   - `src-tauri/Cargo.toml` and the LlamaPilot entry in `src-tauri/Cargo.lock`;
   - `src-tauri/tauri.conf.json`.
2. Move the relevant entries from **Unreleased** in `CHANGELOG.md` into a dated version section.
3. Run `./scripts/check-version-sync.ps1` and the full verification sequence from
   `CONTRIBUTING.md`.
4. Commit the version and changelog together.

The application version, not the workflow, is the source of truth. The release workflow refuses a
tag that does not exactly match it.

## 2. Tag and publish

After the release commit is on `main`:

```powershell
git tag -a v0.1.5 -m "LlamaPilot v0.1.5"
git push origin main
git push origin v0.1.5
```

Replace `0.1.5` with the prepared version. The tag starts the Windows release workflow, which runs
all frontend and Rust checks before building the NSIS and MSI packages. A failed check must be
fixed in a new commit and a new version; do not move a published release tag.

## 3. Verify the published artifacts

On a clean Windows machine or VM:

1. Install using the NSIS executable and launch the application.
2. Confirm the displayed version and LlamaPilot branding.
3. Exercise the first-run path through toolchain detection and model-directory selection.
4. Uninstall and verify that user-owned sources, models, and external workspaces were not removed.
5. Repeat the installation check with the MSI package when it is part of the release.

Production installers should be Authenticode-signed by the distributor. Signing credentials must
remain in the release environment and must never be committed to the repository.
