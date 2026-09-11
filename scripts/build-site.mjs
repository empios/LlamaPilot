import { readFileSync, writeFileSync, mkdirSync, cpSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const releaseFile = process.argv[2] || path.join(root, "website/release.json");
const release = JSON.parse(readFileSync(releaseFile, "utf8"));
if (
  release.draft ||
  release.prerelease ||
  !/^v\d+\.\d+\.\d+$/.test(release.tag_name)
) {
  throw new Error("The landing page requires a published stable version.");
}
const version = release.tag_name.slice(1);
const releaseUrl = `https://github.com/empios/LlamaPilot/releases/tag/${release.tag_name}`;
if (release.html_url !== releaseUrl)
  throw new Error("Unexpected release origin.");
const assetNames = {
  macArm: `LlamaPilot_${version}_aarch64.dmg`,
  macIntel: `LlamaPilot_${version}_x64.dmg`,
  windowsExe: `LlamaPilot_${version}_x64-setup.exe`,
  windowsMsi: `LlamaPilot_${version}_x64_en-US.msi`,
  linuxAppImage: `LlamaPilot_${version}_amd64.AppImage`,
  linuxDeb: `LlamaPilot_${version}_amd64.deb`,
  checksums: "SHA256SUMS.txt",
};
const replacements = { version: release.tag_name, releaseUrl };
for (const [key, name] of Object.entries(assetNames)) {
  const matches = release.assets.filter((asset) => asset.name === name);
  const url = `https://github.com/empios/LlamaPilot/releases/download/${release.tag_name}/${name}`;
  if (matches.length !== 1 || matches[0].browser_download_url !== url) {
    throw new Error(`Missing or invalid release download: ${name}`);
  }
  replacements[key] = url;
}
const escape = (value) =>
  value.replace(
    /[&<>"']/g,
    (char) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        char
      ],
  );
let html = readFileSync(path.join(root, "website/index.html"), "utf8");
html = html.replace(/\{\{(\w+)\}\}/g, (_, key) => {
  if (!(key in replacements)) throw new Error(`Unknown template key: ${key}`);
  return escape(replacements[key]);
});
if (/\{\{|\}\}/.test(html)) throw new Error("Unresolved template placeholder.");
const output = path.join(root, "website-dist");
mkdirSync(output, { recursive: true });
cpSync(path.join(root, "website/assets"), path.join(output, "assets"), {
  recursive: true,
});
for (const name of ["styles.css", "app.js"])
  cpSync(path.join(root, "website", name), path.join(output, name));
writeFileSync(path.join(output, "index.html"), html);
writeFileSync(path.join(output, ".nojekyll"), "");
console.log(
  `Built ${release.tag_name} landing page with all six installers and checksums in ${output}`,
);
