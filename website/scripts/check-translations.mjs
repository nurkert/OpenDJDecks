#!/usr/bin/env node
// Guides in the two languages must stay in step: same files, front matter in both, same
// number of ## sections and screenshots. Missing translations or front matter fail the build;
// a different section count is reported as a warning (wording can legitimately differ).
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

import { guideOrder, guideRoot } from '../src/lib/docs-catalog.mjs';
import { parseFrontMatter } from '../src/lib/search-index.mjs';

let failed = false;
let warnings = 0;
const required = ['title', 'description'];
const sections = (text) => (text.match(/^## /gm) ?? []).length;
const images = (text) => (text.match(/!\[/g) ?? []).length;

for (const slug of guideOrder) {
  const files = { en: path.join(guideRoot, 'en', `${slug}.md`), de: path.join(guideRoot, 'de', `${slug}.md`) };
  const parsed = {};
  for (const [lang, file] of Object.entries(files)) {
    if (!existsSync(file)) {
      console.error(`MISSING: ${lang}/${slug}.md`);
      failed = true;
      continue;
    }
    parsed[lang] = parseFrontMatter(readFileSync(file, 'utf8'));
    for (const field of required) {
      if (!parsed[lang].data[field]) {
        console.error(`NO ${field}: ${lang}/${slug}.md`);
        failed = true;
      }
    }
    if (!(parsed[lang].data.keywords?.length > 0)) {
      console.error(`NO keywords: ${lang}/${slug}.md`);
      failed = true;
    }
    if (!(parsed[lang].data.features?.length >= 0)) failed = true;
  }
  if (parsed.en && parsed.de) {
    const a = sections(parsed.en.body);
    const b = sections(parsed.de.body);
    if (a !== b) {
      console.warn(`warning: ${slug}: ${a} sections in English, ${b} in German`);
      warnings += 1;
    }
    if (images(parsed.en.body) !== images(parsed.de.body)) {
      console.warn(`warning: ${slug}: different number of images`);
      warnings += 1;
    }
  }
}
if (failed) process.exit(1);
console.log(`Translations OK: ${guideOrder.length} guides in both languages, ${warnings} warnings.`);
