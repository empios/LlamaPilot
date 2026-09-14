# Release process

The desktop release workflow builds Windows x64 NSIS/MSI, Apple Silicon and Intel macOS DMGs,
and Linux x64 AppImage/Debian packages from the same version tag.

## Prepare

1. Update package.json, both root versions in package-lock.json, Cargo.toml, the LlamaPilot entry
   in Cargo.lock, and tauri.conf.json together. Platform overrides must not duplicate the version.
2. Update CHANGELOG.md and run the verification commands from CONTRIBUTING.md.
3. Complete and record the platform qualification checklist in PLATFORM_SUPPORT.md. Distinguish
   tested GPU/desktop combinations from build-only results.
4. Merge the verified release commit to main. Published tags and assets are immutable.

```sh
npm run check:version
git tag -a v0.3.0 -m "LlamaPilot v0.3.0"
git push origin v0.3.0
```

Use the version actually prepared. Manual workflow dispatch requires an existing matching tag;
its commit must be reachable from main. The workflow cannot silently create a release from an
arbitrary branch. All build jobs must succeed, and prepare-release.mjs checks every required
architecture/format and rejects colliding asset names before producing SHA256SUMS.txt.

One final job creates a draft, uploads all assets, and publishes it. A failed build leaves the
previous release untouched. Failed uploads leave a draft that can be retried. A rerun refuses to
overwrite an already published release; fixes require a new version.

For an unpublished tag whose application source is correct, packaging-validation or publishing
script fixes can be retried by dispatching the release workflow from `main` with that existing
tag. Application source, version, and build configuration still come from the immutable tag;
release validation tools come from the dispatch commit in `.release-tools` and their tests run
before packaging. This allows pipeline repairs without moving version tags. Changes to the app
or its build configuration require a new version tag.

## macOS signing

The macOS configuration defaults to ad-hoc signing (`-`) without notarization. APPLE_SIGNING_IDENTITY overrides it when a Developer ID is configured. Configure
protected repository/release credentials for Developer ID distribution:
APPLE_CERTIFICATE (base64 PKCS#12), APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY,
APPLE_ID, APPLE_PASSWORD (app-specific), and APPLE_TEAM_ID.
Never commit credentials. Tauri imports the identity, signs with hardened runtime, submits for
notarization, and staples the result when those credentials are supplied. Verify both architecture
DMGs and update the release's signing statement after checking the actual artifacts.

Windows Authenticode remains a separate distributor configuration. Linux DEBs have SHA-256
integrity hashes; AppImage updates additionally have a Minisign signature as described below.

## Updater signing and first release

The release workflow now requires updater signing. This is separate from Windows Authenticode
and Apple code signing/notarization. Ordinary development/CI builds remain possible without a
production key; they show manual release downloads and never automatically install an update.

1. Generate one production key using `npm run tauri signer generate -- -w <secure-path-outside-repository>`.
   Choose a password, retain an encrypted backup of the key and password, and never commit them.
2. In the GitHub repository's Actions settings, set repository **variable**
   `TAURI_UPDATER_PUBLIC_KEY` to the complete contents of the generated `.pub` file.
3. Set repository **secrets** `TAURI_SIGNING_PRIVATE_KEY` (complete private-key file contents)
   and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (its password). Do not put a machine-local key path
   into the GitHub secret: runners need the actual key contents. Do not print these values in logs.
4. Keep this key stable across releases. `configure-updater.mjs` generates an ignored public
   config overlay with `createUpdaterArtifacts: true` and the public key. The build receives the
   private key only through its environment. Missing credentials fail the release build.
5. Prepare a new version greater than 0.3.0 and complete the platform qualification below.
   The first updater-enabled version must be installed manually. Test its upgrade to a second
   version before claiming automatic updates are qualified.

Each target records version, target and package SHA-256 hashes in `updater-build.json` immediately
after building. The final publishing job verifies this provenance and every updater signature
against the configured public key. It renames macOS archives with architecture/version suffixes,
builds one complete `latest.json`, and uploads packages, signatures, manifest and checksums to a
draft before publishing. The default stable endpoint is
`https://github.com/empios/LlamaPilot/releases/latest/download/latest.json`.
NSIS is the Windows updater target; MSI packages are signed but retained for manual deployments.
Published releases must remain immutable. A rerun uses fresh downloaded artifacts and refuses
to overwrite an already published release.

The NSIS post-install hook writes `llamapilot-install-kind`; only binaries identified by Tauri
as NSIS with that installation marker enable Windows in-app installation. This also rejects an
MSI binary placed into a directory with a stale NSIS marker. The uninstaller removes the marker. An unmarked older install needs
one manual upgrade. Installed macOS apps and writable AppImages support in-app installation;
mounted DMGs, DEB packages and unmarked EXEs use manual downloads. Keep the app identifier and
data paths stable so upgrades retain settings, profiles, model locations and runtime records.

### Qualification before publication

Run `npm run test:release`, the usual frontend/native CI checks, and a real two-version upgrade
on each supported package/architecture. A successful build is not an installed-upgrade test.
For qualification, use a separate test signing key, test app identifier and test HTTPS endpoint;
do not point production installations to a test feed. Never publish installers signed with
temporary test keys as a production release.

- Windows NSIS: per-user install, admin-required install, denied UAC, restart, retained profiles,
  new version display, and no new server/build admitted while the installer is starting.
- macOS ARM64/x64: app copied into Applications, ad-hoc and Developer ID/notarized distribution,
  permissions, restart, and retained data. Mounted DMG must offer manual installation.
- Linux AppImage: writable and non-writable location, restart and retained executable permissions.
- MSI/DEB/standalone: release notification and manual downloads, without a package-type conversion.
- All: offline/timeout, invalid signature, incomplete download, missing target, deferred restart,
  server/build/benchmark/model-download active, open dialogs and unsaved settings.

### Recovery

Download/verification errors leave the installed application untouched. Close or finish active
work and retry from Settings. A downloaded update is kept only in memory; restarting the app
requires downloading it again. If installation itself is interrupted, run the installer for
the intended version from the official release. Keep application data and model folders intact.

There is no automatic rollback. Repair a faulty published version with a higher version number.
Before manually downgrading, verify data-schema compatibility and back up the data directory.
Future schema migrations must retain a recoverable copy before rewriting persisted data.
If rotating a signing key, distribute a transition release trusted by the old key before using
the new one. Losing the old key before that transition requires a manual reinstall for affected
clients; replacing the repository variable alone cannot update the key embedded in their apps.

## Local macOS delivery

```sh
npm ci
APPLE_SIGNING_IDENTITY=- npm run tauri build -- --bundles dmg
shasum -a 256 src-tauri/target/release/bundle/dmg/*.dmg
```

Inspect with `hdiutil verify`, mount read-only, verify architecture and `codesign --verify --deep
--strict`, and launch the copied app from Finder. Keep local working-tree builds distinguishable
from the assets produced from a published tag. The implementation plan records local validation.
