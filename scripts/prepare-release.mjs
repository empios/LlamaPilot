import { readdirSync, readFileSync, mkdirSync, copyFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import path from 'node:path';
const root = path.resolve(process.argv[2] ?? 'release-artifacts');
const files = [];
function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === 'final') continue;
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(p);
    else if (/\.(dmg|deb|AppImage|exe|msi)$/.test(entry.name)) files.push(p);
  }
}
walk(root);
for (const [target, formats] of [
  ['aarch64-apple-darwin', ['dmg']], ['x86_64-apple-darwin', ['dmg']],
  ['x86_64-unknown-linux-gnu', ['deb', 'AppImage']], ['x86_64-pc-windows-msvc', ['exe', 'msi']],
]) {
  for (const ext of formats) if (!files.some((p) => p.includes(`installers-${target}${path.sep}`) && p.endsWith(`.${ext}`))) {
    throw new Error(`Missing ${target} ${ext} package`);
  }
}
const out = path.join(root, 'final'); mkdirSync(out, { recursive: true });
const names = new Set(); const sums = [];
for (const p of files.sort()) {
  const name = path.basename(p);
  if (names.has(name)) throw new Error(`Duplicate asset name: ${name}`);
  names.add(name); copyFileSync(p, path.join(out, name));
  sums.push(`${createHash('sha256').update(readFileSync(p)).digest('hex')}  ${name}`);
}
writeFileSync(path.join(out, 'SHA256SUMS.txt'), sums.join('\n') + '\n');
console.log(`Prepared ${files.length} installers and SHA256SUMS.txt`);
