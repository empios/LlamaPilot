import { appendFileSync } from 'node:fs';
const names = ['APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_SIGNING_IDENTITY',
  'APPLE_ID', 'APPLE_PASSWORD', 'APPLE_TEAM_ID'];
const entries = names.map((name) => [name, process.env[`RELEASE_${name}`] ?? '']);
if (entries.every(([, value]) => value === '')) {
  console.log('Using ad-hoc signing without notarization.');
} else {
  for (const [name, value] of entries) {
    if (!value) throw new Error(`Incomplete Apple signing configuration: ${name} is missing`);
    if (/[\r\n]/.test(value)) throw new Error(`${name} must be a single-line value`);
  }
  if (!process.env.GITHUB_ENV) throw new Error('GITHUB_ENV is unavailable');
  appendFileSync(process.env.GITHUB_ENV, entries.map(([name, value]) => `${name}=${value}\n`).join(''));
  console.log('Developer ID signing and notarization configured.');
}
