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

## macOS signing

Local builds without an identity use Tauri's ad-hoc signing and are not notarized. Configure
protected repository/release credentials for Developer ID distribution:
APPLE_CERTIFICATE (base64 PKCS#12), APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY,
APPLE_ID, APPLE_PASSWORD (app-specific), and APPLE_TEAM_ID.
Never commit credentials. Tauri imports the identity, signs with hardened runtime, submits for
notarization, and staples the result when those credentials are supplied. Verify both architecture
DMGs and update the release's signing statement after checking the actual artifacts.

Windows Authenticode remains a separate distributor configuration. Linux assets have SHA-256
integrity hashes; those are not a publisher signature.

## Local macOS delivery

```sh
npm ci
npm run tauri build -- --bundles dmg
shasum -a 256 src-tauri/target/release/bundle/dmg/*.dmg
```

Inspect with `hdiutil verify`, mount read-only, verify architecture and `codesign --verify --deep
--strict`, and launch the copied app from Finder. Keep local working-tree builds distinguishable
from the assets produced from a published tag. The implementation plan records local validation.
