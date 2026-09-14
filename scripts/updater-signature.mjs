import { createHash, createPublicKey, verify } from 'node:crypto';

// Release preflight for Tauri's base64-encoded Minisign text files. The official
// updater independently verifies again when the application downloads an update.
function decode(value) {
  if (typeof value !== 'string' || !/^[A-Za-z0-9+/]+={0,2}$/.test(value.trim())) throw new Error('Invalid base64 signing data');
  return Buffer.from(value.trim(), 'base64');
}

export function readPublicKey(encoded) {
  const lines = decode(encoded).toString('utf8').trim().split(/\r?\n/);
  const raw = decode(lines[1]);
  if (!lines[0].startsWith('untrusted comment: ') || raw.length !== 42 || raw.subarray(0, 2).toString() !== 'Ed') throw new Error('Invalid updater public key');
  const key = createPublicKey({ key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), raw.subarray(10)]), format: 'der', type: 'spki' });
  return { raw, key };
}

export function verifyUpdaterSignature(bytes, signature, encodedKey) {
  const { raw: publicKey, key } = readPublicKey(encodedKey);
  const lines = decode(signature).toString('utf8').trim().split(/\r?\n/);
  if (lines.length !== 4 || !lines[0].startsWith('untrusted comment: ') || !lines[2].startsWith('trusted comment: ')) throw new Error('Invalid updater signature');
  const raw = decode(lines[1]);
  const global = decode(lines[3]);
  if (raw.length !== 74 || global.length !== 64 || !raw.subarray(2, 10).equals(publicKey.subarray(2, 10))) throw new Error('Wrong updater signing key');
  const algorithm = raw.subarray(0, 2).toString();
  if (!['Ed', 'ED'].includes(algorithm)) throw new Error('Unsupported signature algorithm');
  const message = algorithm === 'ED' ? createHash('blake2b512').update(bytes).digest() : bytes;
  const valid = verify(null, message, key, raw.subarray(10))
    && verify(null, Buffer.concat([raw.subarray(10), Buffer.from(lines[2].slice(17))]), key, global);
  if (!valid) throw new Error('Updater signature verification failed');
}
