import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// The classifier lives in TypeScript; its patterns are what matters, so the test reads them.
const source = readFileSync(new URL('../src/lib/releases.ts', import.meta.url), 'utf8');
const body = source.slice(source.indexOf('const name ='), source.indexOf('interface ApiAsset'));
const stripped = body.replace(/: \[Kind, Platform, RegExp\]\[\]/, '');
const module_ = new Function(`${stripped.replaceAll('new RegExp', 'new RegExp')}; return { kinds };`);
const { kinds } = module_();

const kindOf = (name) => kinds.find(([, , re]) => re.test(name))?.[0];

const current = {
  'OpenDJDecks-0.4.0-macos-arm64.dmg': 'dmgArm',
  'OpenDJDecks-0.4.0-macos-x86_64.dmg': 'dmgIntel',
  'OpenDJDecks-0.4.0-windows-x86_64.zip': 'zip',
  'opendjdecks_0.4.0_amd64.deb': 'debAmd',
  'opendjdecks_0.4.0_arm64.deb': 'debArm',
  'OpenDJDecks-0.4.0-x86_64.AppImage': 'appimageX86',
  'OpenDJDecks-0.4.0-aarch64.AppImage': 'appimageArm',
  'opendjdecks-0.4.0-linux-x86_64.tar.gz': 'tarX86',
  'opendjdecks-0.4.0-linux-aarch64.tar.gz': 'tarArm',
  'opendjdecks-pi-0.4.0.tar.gz': 'pi',
};
const legacy = {
  'OpenDecks-Session-0.3.0-macos-arm64.dmg': 'dmgArm',
  'OpenDecks-Session-0.3.0-macos-x86_64.dmg': 'dmgIntel',
  'OpenDecks-Session-0.3.0-windows-x86_64.zip': 'zip',
  'opendecks-session_0.3.0_amd64.deb': 'debAmd',
  'opendecks-session_0.3.0_arm64.deb': 'debArm',
  'OpenDecks-Session-0.3.0-x86_64.AppImage': 'appimageX86',
  'OpenDecks-Session-0.3.0-aarch64.AppImage': 'appimageArm',
  'opendecks-session-0.3.0-linux-x86_64.tar.gz': 'tarX86',
  'opendecks-session-0.3.0-linux-aarch64.tar.gz': 'tarArm',
  'opendecks-pi-0.3.0.tar.gz': 'pi',
};

test('current asset names are recognised', () => {
  for (const [name, kind] of Object.entries(current)) assert.equal(kindOf(name), kind, name);
});
test('asset names of the release under the old name are recognised', () => {
  for (const [name, kind] of Object.entries(legacy)) assert.equal(kindOf(name), kind, name);
});
test('checksums and unrelated files are not treated as downloads', () => {
  assert.equal(kindOf('SHA256SUMS.txt'), undefined);
  assert.equal(kindOf('notes.md'), undefined);
});
