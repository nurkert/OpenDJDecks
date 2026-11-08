// The search test: runs the very options the browser uses against the records the build
// produces from the Markdown. "Searching these words returns the right guide first."
import assert from 'node:assert/strict';
import test from 'node:test';

import MiniSearch from 'minisearch';

import { dedupeByPage, miniSearchOptions } from '../src/lib/search-core.mjs';
import { buildRecords } from '../src/lib/search-index.mjs';

function searcher(lang) {
  const engine = new MiniSearch(miniSearchOptions());
  const records = buildRecords(lang);
  engine.addAll(records);
  return { records, first: (query) => dedupeByPage(engine.search(query))[0]?.href ?? '' };
}

const de = searcher('de');
const en = searcher('en');

const hits = [
  // [language, query, guide slug expected first]
  ['de', 'zuschneidn', 'editing-tracks'],
  ['de', 'zuschneiden', 'editing-tracks'],
  ['de', 'track kürzen', 'editing-tracks'],
  ['de', 'cutten', 'editing-tracks'],
  ['de', 'beatgrit', 'beatgrid'],
  ['de', 'beatgrid korrigieren', 'beatgrid'],
  ['de', 'hot cue setzen', 'cues-and-loops'],
  ['de', 'stick synchronisieren', 'usb-sticks'],
  ['de', 'stems trennen', 'stems'],
  ['de', 'aufnehmen', 'recording'],
  ['de', 'tasten belegen', 'preferences'],
  ['de', 'rekordbox xml', 'importing-your-library'],
  ['de', 'licht dmx', 'light-show'],
  ['de', 'midi learn', 'controllers'],
  ['en', 'cut a track', 'editing-tracks'],
  ['en', 'trim', 'editing-tracks'],
  ['en', 'beatgrit', 'beatgrid'],
  ['en', 'drift', 'beatgrid'],
  ['en', 'memory cue', 'cues-and-loops'],
  ['en', 'smart playlist', 'library'],
  ['en', 'usb stick sync', 'usb-sticks'],
  ['en', 'stems', 'stems'],
  ['en', 'sampler sequencer', 'sampler'],
  ['en', 'record my mix', 'recording'],
  ['en', 'rekordbox xml', 'importing-your-library'],
  ['en', 'dmx lights', 'light-show'],
];

for (const [lang, query, slug] of hits) {
  test(`${lang}: "${query}" finds ${slug} first`, () => {
    const s = lang === 'de' ? de : en;
    const href = s.first(query);
    assert.ok(href.includes(`/wiki/${slug}/`), `got "${href}"`);
  });
}

test('section hits jump to their heading', () => {
  const engine = new MiniSearch(miniSearchOptions());
  engine.addAll(buildRecords('en'));
  const results = dedupeByPage(engine.search('AUTO-FIX'));
  assert.ok(results.some((r) => r.href.includes('#')), 'at least one result carries a heading anchor');
});

test('the index is small enough to load on first focus', () => {
  for (const s of [de, en]) {
    const kb = JSON.stringify(s.records).length / 1024;
    assert.ok(kb < 500, `index ${kb.toFixed(0)} KB`);
  }
});
