import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';

import { guideRoot, docsRoot, repoRoot } from '../src/lib/docs-catalog.mjs';
import rehypeDocs, { headingId } from '../src/lib/rehype-docs.mjs';
import remarkDocs, { rewriteDocLink } from '../src/lib/remark-docs.mjs';

const guideEn = path.join(guideRoot, 'en', 'beatgrid.md');
const guideDe = path.join(guideRoot, 'de', 'beatgrid.md');
const dev = path.join(docsRoot, 'mcp.md');

test('a link to another guide resolves to its page in the same language', () => {
  assert.equal(rewriteDocLink('editing-tracks.md#the-result', guideEn), '/en/wiki/editing-tracks/#the-result');
  assert.equal(rewriteDocLink('editing-tracks.md', guideDe), '/wiki/editing-tracks/');
});

test('a guide can link to a developer doc in its own language', () => {
  assert.equal(rewriteDocLink('../../keyboard.md', guideEn), '/en/wiki/dev/keyboard/');
  assert.equal(rewriteDocLink('../../keyboard.md#all-defaults', guideDe), '/wiki/dev/keyboard/#all-defaults');
});

test('other repository files point at GitHub', () => {
  const blob = rewriteDocLink('../DESIGN.md', dev);
  assert.match(blob, /github\.com\/nurkert\/OpenDJDecks\/blob\/main\/DESIGN\.md$/);
});

test('external links and anchors stay untouched', () => {
  for (const url of ['https://example.org/a.md', '#here', 'mailto:x@y']) assert.equal(rewriteDocLink(url, guideEn), url);
});

test('remark plugin rewrites link nodes and removes HTML comments', () => {
  const tree = {
    type: 'root',
    children: [
      { type: 'paragraph', children: [{ type: 'link', url: 'stems.md', children: [] }] },
      { type: 'html', value: '<!-- note -->' },
    ],
  };
  remarkDocs()(tree, { path: guideEn });
  assert.equal(tree.children.length, 1);
  assert.equal(tree.children[0].children[0].url, '/en/wiki/stems/');
});

test('heading ids are stable and unique', () => {
  const seen = new Map();
  assert.equal(headingId('Set a hot cue', seen), 'set-a-hot-cue');
  assert.equal(headingId('Set a hot cue', seen), 'set-a-hot-cue-1');
  assert.equal(headingId('Schritt für Schritt', new Map()), 'schritt-fuer-schritt');
});

const h = (name, props, children = []) => ({ type: 'element', tagName: name, properties: props ?? {}, children });
const t = (value) => ({ type: 'text', value });

test('rehype: headings get ids, tables are framed, admonitions become notes', () => {
  const tree = {
    type: 'root',
    children: [
      h('h2', {}, [t('A heading')]),
      h('table', {}, [h('tr', {}, [h('td', {}, [t('x')])])]),
      h('blockquote', {}, [h('p', {}, [t('[!TIP] Do this.')])]),
    ],
  };
  rehypeDocs()(tree);
  assert.equal(tree.children[0].properties.id, 'a-heading');
  assert.equal(tree.children[1].tagName, 'div');
  assert.equal(tree.children[1].children[0].tagName, 'table');
  const note = tree.children[2];
  assert.equal(note.tagName, 'aside');
  assert.deepEqual(note.properties.className, ['note', 'note--tip']);
  assert.equal(note.children[0].children[0].value, 'Do this.');
});

test('the repository root is resolved', () => {
  assert.ok(repoRoot.endsWith(path.sep));
});
