import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (name) => readFileSync(path.join(root, name), 'utf8');
const pkg = JSON.parse(read('package.json'));
const lock = JSON.parse(read('package-lock.json'));
const tauri = JSON.parse(read('src-tauri/tauri.conf.json'));
const cargo = read('src-tauri/Cargo.toml').split('[package]')[1]?.split(/\n\[/)[0];
const name = cargo?.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
const version = cargo?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const entry = read('src-tauri/Cargo.lock').split('[[package]]')
  .find((section) => section.match(/^name\s*=\s*"([^"]+)"/m)?.[1] === name);
const versions = [pkg.version, lock.version, lock.packages?.['']?.version, tauri.version,
  version, entry?.match(/^version\s*=\s*"([^"]+)"/m)?.[1]];
if (!pkg.version || versions.some((v) => v !== pkg.version)) throw new Error(`Version mismatch: ${versions.join(', ')}`);
if ([lock.name, lock.packages?.['']?.name, name].some((n) => n !== pkg.name)) throw new Error('Package names differ');
const tag = process.argv[2];
if (tag && tag !== `v${pkg.version}`) throw new Error(`Tag ${tag} does not match v${pkg.version}`);
console.log(`Version metadata is synchronized at ${pkg.version}.`);
