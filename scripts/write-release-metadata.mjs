import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { releaseTargets } from './prepare-release.mjs';

export function writeReleaseMetadata(directory, { target, version }) {
  const spec = releaseTargets.find((entry) => entry.target === target);
  if (!spec) throw new Error('Unknown release target');
  const files = new Map();
  function walk(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name);
      // Packaging leaves staging trees and links beside the final packages. Do not
      // follow them or mistake their contents for files uploaded to the release.
      if (entry.isSymbolicLink()) continue;
      if (entry.isDirectory() && !/\.(app|AppDir)$/.test(entry.name)) walk(file);
      else if (entry.isFile() && /\.(dmg|deb|AppImage|exe|msi|app\.tar\.gz)$/.test(entry.name)) {
        if (files.has(entry.name)) throw new Error(`Duplicate package name: ${entry.name}`);
        files.set(entry.name, file);
      }
    }
  }
  walk(directory);
  const packages = {};
  for (const ext of spec.formats) {
    const matches = [...files].filter(([name]) => name.endsWith(`.${ext}`));
    if (matches.length !== 1) throw new Error(`Expected exactly one ${target} ${ext} package after build; found ${matches.length}`);
    const [name, file] = matches[0];
    const bytes = readFileSync(file);
    if (!bytes.length) throw new Error(`Empty package: ${name}`);
    if (!['dmg', 'deb'].includes(ext) && !readFileSync(`${file}.sig`, 'utf8').trim()) throw new Error(`Empty updater signature: ${name}`);
    packages[name] = createHash('sha256').update(bytes).digest('hex');
  }
  writeFileSync(path.join(directory, 'updater-build.json'), JSON.stringify({ target, version, packages }) + '\n');
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const target = process.argv[2];
  const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
  writeReleaseMetadata(path.join('src-tauri', 'target', target, 'release', 'bundle'), { target, version });
}
