# LlamaPilot landing page

The public site is deployed to https://empios.github.io/LlamaPilot/ using GitHub Pages.
It is an independent static site; desktop application code and build output are not published.

## Preview locally

```sh
npm run site:build
python3 -m http.server 4174 --bind 127.0.0.1 --directory website-dist
```

Open http://127.0.0.1:4174. The local build uses the checked-in release.json as an offline
snapshot. No dependency install is needed for `node scripts/build-site.mjs`.

The Pages workflow reads the latest published stable GitHub release. The build validates its
repository, version, and all six installer filenames and checksum download before generating the
HTML. Invalid or incomplete release metadata fails the build and leaves the live deployment intact.
Download links are ordinary HTML anchors and work without JavaScript.

Changes to website/, the site builder, or the Pages workflow on main trigger deployment.
Publishing a release also updates the site. A successful desktop release workflow triggers Pages
separately because events created with GITHUB_TOKEN do not generally trigger another workflow.
The deployment can also be started manually with the Deploy landing page workflow.

The Geist font is self-hosted with its OFL license in assets/GEIST-LICENSE.txt. Product screenshots
and the app icon come from this repository. The neural globe is an original SVG illustration.
