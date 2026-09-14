import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import path from 'node:path';

const target = process.argv[2];
if (!['x86_64-pc-windows-msvc', 'aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu'].includes(target)) throw new Error('Unknown release target');
const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
const directory = path.join('src-tauri', 'target', target, 'release', 'bundle');
mkdirSync(directory, { recursive: true });
const packages = {};
function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(file);
    else if (/\.(dmg|deb|AppImage|exe|msi|app\.tar\.gz)$/.test(entry.name)) {
      if (packages[entry.name]) throw new Error(`Duplicate package name: ${entry.name}`);
      packages[entry.name] = createHash('sha256').update(readFileSync(file)).digest('hex');
    }
  }
}
walk(directory);
writeFileSync(path.join(directory, 'updater-build.json'), JSON.stringify({ target, version, packages }) + '\n');
