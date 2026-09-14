import test from 'node:test';
import assert from 'node:assert/strict';
import { generateKeyPairSync, randomBytes, sign, createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, unlinkSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { prepareRelease } from './prepare-release.mjs';
import { verifyUpdaterSignature } from './updater-signature.mjs';
import { writeReleaseMetadata } from './write-release-metadata.mjs';

test('macOS packaging enables both the disk image and the updater app target', () => {
  const config = JSON.parse(readFileSync(new URL('../src-tauri/tauri.macos.conf.json', import.meta.url), 'utf8'));
  assert.ok(config.bundle.targets.includes('dmg'));
  assert.ok(config.bundle.targets.includes('app'), 'DMG alone does not generate a signed updater archive');
});

function signer() {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const id = randomBytes(8);
  const raw = Buffer.concat([Buffer.from('Ed'), id, publicKey.export({ format: 'der', type: 'spki' }).subarray(-32)]);
  return {
    key: Buffer.from(`untrusted comment: test key\n${raw.toString('base64')}\n`).toString('base64'),
    sign(bytes) {
      const signature = sign(null, createHash('blake2b512').update(bytes).digest(), privateKey);
      const signed = Buffer.concat([Buffer.from('ED'), id, signature]);
      const comment = 'test package';
      const global = sign(null, Buffer.concat([signature, Buffer.from(comment)]), privateKey);
      return Buffer.from(`untrusted comment: test\n${signed.toString('base64')}\ntrusted comment: ${comment}\n${global.toString('base64')}\n`).toString('base64');
    },
  };
}

function fixture() {
  const root = mkdtempSync(path.join(tmpdir(), 'llamapilot-release-test-'));
  const keys = signer();
  const version = '0.4.0';
  const files = [];
  for (const [target, arch, extensions] of [
    ['x86_64-pc-windows-msvc', 'x64', ['exe', 'msi']],
    ['aarch64-apple-darwin', 'aarch64', ['dmg', 'app.tar.gz']],
    ['x86_64-apple-darwin', 'x64', ['dmg', 'app.tar.gz']],
    ['x86_64-unknown-linux-gnu', 'amd64', ['deb', 'AppImage']],
  ]) {
    const directory = path.join(root, `installers-${target}`);
    mkdirSync(directory);
    const packages = {};
    for (const ext of extensions) {
      const name = ext === 'app.tar.gz' ? 'LlamaPilot.app.tar.gz' : `LlamaPilot_${version}_${arch}_setup.${ext}`;
      const file = path.join(directory, name);
      const bytes = Buffer.from(`test package ${target} ${ext}`);
      writeFileSync(file, bytes);
      packages[name] = createHash('sha256').update(bytes).digest('hex');
      if (!['deb', 'dmg'].includes(ext)) writeFileSync(`${file}.sig`, keys.sign(bytes));
      files.push(file);
    }
    writeFileSync(path.join(directory, 'updater-build.json'), JSON.stringify({ target, version, packages }));
  }
  return { root, files, keys, options: { version, publicKey: keys.key } };
}

test('publishes all target entries, architecture-specific archives and checksums', () => {
  const f = fixture();
  const result = prepareRelease(f.root, f.options);
  assert.equal(Object.keys(result.platforms).length, 4);
  assert.match(result.platforms['windows-x86_64'].url, /\.exe$/);
  assert.notEqual(result.platforms['darwin-x86_64'].url, result.platforms['darwin-aarch64'].url);
  assert.match(readFileSync(path.join(f.root, 'final', 'SHA256SUMS.txt'), 'utf8'), /latest\.json/);
  assert.throws(() => prepareRelease(f.root, f.options), /already exists/);
});

test('missing signature fails before any output is created', () => {
  const f = fixture();
  unlinkSync(f.files[0] + '.sig');
  assert.throws(() => prepareRelease(f.root, f.options));
  assert.equal(existsSync(path.join(f.root, 'final')), false);
});

test('build provenance rejects missing macOS updater archives and signatures before upload', () => {
  for (const suffix of ['', '.sig']) {
    const f = fixture();
    const directory = path.join(f.root, 'installers-aarch64-apple-darwin');
    unlinkSync(path.join(directory, 'updater-build.json'));
    unlinkSync(path.join(directory, `LlamaPilot.app.tar.gz${suffix}`));
    assert.throws(() => writeReleaseMetadata(directory, { target: 'aarch64-apple-darwin', version: '0.4.0' }), /app\.tar\.gz/);
    assert.equal(existsSync(path.join(directory, 'updater-build.json')), false);
  }
});

test('build provenance for complete signed packages is accepted by the publisher', () => {
  const f = fixture();
  for (const target of ['x86_64-pc-windows-msvc', 'aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu']) {
    writeReleaseMetadata(path.join(f.root, `installers-${target}`), { target, version: '0.4.0' });
  }
  assert.equal(Object.keys(prepareRelease(f.root, f.options).platforms).length, 4);
});

test('build provenance ignores AppImage staging directories and never follows symlinks', () => {
  const f = fixture();
  const directory = path.join(f.root, 'installers-x86_64-unknown-linux-gnu');
  const staging = path.join(directory, 'LlamaPilot.AppDir');
  mkdirSync(staging);
  writeFileSync(path.join(staging, 'intermediate.AppImage'), 'not a release package');
  const unrelated = path.join(f.root, 'unrelated');
  mkdirSync(unrelated);
  writeFileSync(path.join(unrelated, 'external.AppImage'), 'not a release package');
  symlinkSync(unrelated, path.join(directory, 'staging-link'), 'junction');
  writeReleaseMetadata(directory, { target: 'x86_64-unknown-linux-gnu', version: '0.4.0' });
  const metadata = JSON.parse(readFileSync(path.join(directory, 'updater-build.json'), 'utf8'));
  assert.deepEqual(Object.keys(metadata.packages).sort(), [
    'LlamaPilot_0.4.0_amd64_setup.AppImage', 'LlamaPilot_0.4.0_amd64_setup.deb',
  ]);
});

test('corrupted download and wrong signing key are rejected', () => {
  const keys = signer();
  const bytes = Buffer.from('original package');
  const signature = keys.sign(bytes);
  verifyUpdaterSignature(bytes, signature, keys.key);
  assert.throws(() => verifyUpdaterSignature(Buffer.from('corrupted'), signature, keys.key), /verification failed/);
  assert.throws(() => verifyUpdaterSignature(bytes, signature, signer().key), /Wrong updater/);
});

test('rejects tampered trusted comment', () => {
  const keys = signer();
  const bytes = Buffer.from('package');
  const altered = Buffer.from(Buffer.from(keys.sign(bytes), 'base64').toString().replace('trusted comment: test package', 'trusted comment: altered')).toString('base64');
  assert.throws(() => verifyUpdaterSignature(bytes, altered, keys.key), /verification failed/);
});

test('rejects mismatched version and target metadata', () => {
  for (const wrong of [{ target: 'wrong', version: '0.4.0' }, { target: 'x86_64-pc-windows-msvc', version: '0.3.0' }]) {
    const f = fixture();
    writeFileSync(path.join(f.root, 'installers-x86_64-pc-windows-msvc', 'updater-build.json'), JSON.stringify(wrong));
    assert.throws(() => prepareRelease(f.root, f.options), /mismatch/);
  }
});

test('rejects missing packages, duplicate packages, prereleases and empty signing keys', () => {
  const missing = fixture();
  unlinkSync(missing.files[0]);
  assert.throws(() => prepareRelease(missing.root, missing.options), /exactly one/);
  const duplicate = fixture();
  writeFileSync(duplicate.files[0].replace('.exe', '-copy.exe'), 'duplicate');
  assert.throws(() => prepareRelease(duplicate.root, duplicate.options), /exactly one/);
  assert.throws(() => prepareRelease(duplicate.root, { ...duplicate.options, version: '0.4.0-beta.1' }), /stable/);
  assert.throws(() => prepareRelease(duplicate.root, { ...duplicate.options, publicKey: '' }), /signing data/);
});

test('detects a signed macOS archive accidentally assigned to the other architecture', () => {
  const f = fixture();
  const archives = f.files.filter(file => file.endsWith('.app.tar.gz'));
  writeFileSync(archives[0], readFileSync(archives[1]));
  writeFileSync(archives[0] + '.sig', readFileSync(archives[1] + '.sig'));
  assert.throws(() => prepareRelease(f.root, f.options), /hash mismatch/);
});

test('accepts the upstream Minisign interoperability vector', () => {
  // From minisign-verify 0.2.5's documented example (the updater's verifier).
  const publicKey = Buffer.from('untrusted comment: public key\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3\n').toString('base64');
  const signature = Buffer.from('untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1633700835\tfile:test\tprehashed\nwLMDjy9FLAuxZ3q4NlEvkgtyhrr0gtTu6KC4KBJdITbbOeAi1zBIYo0v4iTgt8jJpIidRJnp94ABQkJAgAooBQ==').toString('base64');
  verifyUpdaterSignature(Buffer.from('test'), signature, publicKey);
  assert.throws(() => verifyUpdaterSignature(Buffer.from('changed'), signature, publicKey), /verification failed/);
});
