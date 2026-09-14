import { readdirSync, readFileSync, mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { verifyUpdaterSignature, readPublicKey } from './updater-signature.mjs';

const targets = [
  { target: 'x86_64-pc-windows-msvc', platform: 'windows-x86_64', arch: 'x64', formats: ['exe', 'msi'], updater: 'exe' },
  { target: 'aarch64-apple-darwin', platform: 'darwin-aarch64', arch: 'aarch64', formats: ['dmg', 'app.tar.gz'], updater: 'app.tar.gz' },
  { target: 'x86_64-apple-darwin', platform: 'darwin-x86_64', arch: 'x64', formats: ['dmg', 'app.tar.gz'], updater: 'app.tar.gz' },
  { target: 'x86_64-unknown-linux-gnu', platform: 'linux-x86_64', arch: 'amd64', formats: ['deb', 'AppImage'], updater: 'AppImage' },
];
function walk(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const file = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Unexpected symlink: ${file}`);
    return entry.isDirectory() ? walk(file) : [file];
  });
}
export function prepareRelease(root, { version, publicKey, notes = '', repository = 'empios/LlamaPilot' }) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) throw new Error('Stable release requires a stable semantic version');
  if (repository !== 'empios/LlamaPilot') throw new Error('Unexpected release repository');
  readPublicKey(publicKey);
  const manifest = { version, notes, pub_date: new Date().toISOString(), platforms: {} };
  const assets = new Map();
  const add = (name, bytes) => {
    if (assets.has(name)) throw new Error(`Duplicate asset name: ${name}`);
    assets.set(name, bytes);
  };
  for (const spec of targets) {
    const files = walk(path.join(root, `installers-${spec.target}`));
    const metadata = files.filter((file) => path.basename(file) === 'updater-build.json');
    if (metadata.length !== 1) throw new Error(`Missing or duplicate build metadata: ${spec.target}`);
    const build = JSON.parse(readFileSync(metadata[0], 'utf8'));
    if (build.target !== spec.target || build.version !== version) throw new Error(`Build target/version mismatch: ${spec.target}`);
    for (const ext of spec.formats) {
      const matches = files.filter((file) => file.endsWith(`.${ext}`));
      if (matches.length !== 1) throw new Error(`Expected exactly one ${spec.target} ${ext} package`);
      const file = matches[0];
      const original = path.basename(file);
      const name = ext === 'app.tar.gz' ? `LlamaPilot_${version}_${spec.platform}.app.tar.gz` : original;
      if (ext !== 'app.tar.gz' && !original.includes(`_${version}_`)) throw new Error(`Package version mismatch: ${original}`);
      if (ext !== 'app.tar.gz' && !new RegExp(`_${spec.arch}(?:[._-]|$)`).test(original)) throw new Error(`Package architecture mismatch: ${original}`);
      const bytes = readFileSync(file);
      if (!bytes.length) throw new Error(`Empty package: ${original}`);
      if (build.packages?.[original] !== createHash('sha256').update(bytes).digest('hex')) throw new Error(`Build package hash mismatch: ${original}`);
      add(name, bytes);
      if (['exe', 'msi', 'AppImage', 'app.tar.gz'].includes(ext)) {
        const signature = readFileSync(`${file}.sig`, 'utf8').trim();
        verifyUpdaterSignature(bytes, signature, publicKey);
        add(`${name}.sig`, Buffer.from(signature + '\n'));
        if (ext === spec.updater) manifest.platforms[spec.platform] = {
          url: `https://github.com/${repository}/releases/download/v${version}/${encodeURIComponent(name)}`,
          signature,
        };
      }
    }
  }
  add('latest.json', Buffer.from(JSON.stringify(manifest, null, 2) + '\n'));
  const sums = [...assets].sort(([a], [b]) => a.localeCompare(b)).map(([name, bytes]) => `${createHash('sha256').update(bytes).digest('hex')}  ${name}`);
  add('SHA256SUMS.txt', Buffer.from(sums.join('\n') + '\n'));
  const out = path.join(root, 'final');
  if (existsSync(out)) throw new Error('Release output already exists; use a fresh artifact directory');
  mkdirSync(out);
  for (const [name, bytes] of assets) writeFileSync(path.join(out, name), bytes);
  return manifest;
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
  if (process.env.RELEASE_TAG !== `v${version}`) throw new Error('Release tag does not match package version');
  prepareRelease(path.resolve(process.argv[2] ?? 'release-artifacts'), {
    version, publicKey: process.env.TAURI_UPDATER_PUBLIC_KEY,
    notes: readFileSync('docs/RELEASE_DOWNLOADS.md', 'utf8'),
  });
  console.log('Prepared complete signed release, latest.json and SHA256SUMS.txt');
}
