#!/usr/bin/env node
// Every in-scope, done feature of the parity list must be explained in a guide.
// A feature counts as covered when a guide (in both languages) lists its id in `features:`.
// Unknown ids, guides that do not exist in the sidebar, and sidebar slugs without a file fail too.
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';

import { parse } from 'smol-toml';

import { guideGroups, guideOrder, guideRoot, repoRoot } from '../src/lib/docs-catalog.mjs';
import { parseFrontMatter } from '../src/lib/search-index.mjs';

const features = parse(readFileSync(path.join(repoRoot, 'tests/parity/features.toml'), 'utf8')).feature;
const known = new Set(features.map((f) => f.id));
const required = features.filter((f) => f.scope === 'in' && f.status === 'done').map((f) => f.id);

const problems = [];
const coveredIn = { de: new Set(), en: new Set() };

for (const lang of ['de', 'en']) {
  for (const slug of guideOrder) {
    const file = path.join(guideRoot, lang, `${slug}.md`);
    if (!existsSync(file)) {
      problems.push(`${lang}/${slug}.md is in the sidebar but does not exist`);
      continue;
    }
    const { data } = parseFrontMatter(readFileSync(file, 'utf8'));
    for (const id of data.features ?? []) {
      if (!known.has(id)) problems.push(`${lang}/${slug}.md lists an unknown feature: ${id}`);
      coveredIn[lang].add(id);
    }
  }
  const stray = readdirSync(path.join(guideRoot, lang)).filter((f) => f.endsWith('.md') && !guideOrder.includes(f.replace(/\.md$/, '')));
  for (const file of stray) problems.push(`${lang}/${file} is not in the sidebar (docs-catalog.mjs)`);
}
const sidebarSlugs = guideGroups.flatMap((g) => g.slugs);
if (new Set(sidebarSlugs).size !== sidebarSlugs.length) problems.push('a guide appears twice in the sidebar');

const missing = (lang) => required.filter((id) => !coveredIn[lang].has(id));
for (const lang of ['de', 'en']) {
  for (const id of missing(lang)) problems.push(`${id} is in scope and done but no ${lang} guide explains it`);
}

if (problems.length) {
  console.error(`Feature coverage FAILED:\n  ${problems.join('\n  ')}`);
  process.exit(1);
}
console.log(`Feature coverage OK: ${required.length} in-scope features explained in ${guideOrder.length} guides per language.`);
