import { writeFileSync } from 'node:fs';
import { readPublicKey } from './updater-signature.mjs';

const publicKey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();
if (!publicKey || !process.env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('Release requires TAURI_UPDATER_PUBLIC_KEY and TAURI_SIGNING_PRIVATE_KEY. Configure the public repository variable and private Actions secret.');
readPublicKey(publicKey);
writeFileSync('src-tauri/tauri.updater.local.json', JSON.stringify({
  bundle: { createUpdaterArtifacts: true },
  plugins: { updater: { pubkey: publicKey } },
}, null, 2) + '\n');
console.log('Configured signed updater artifacts (private key remains only in the environment).');
