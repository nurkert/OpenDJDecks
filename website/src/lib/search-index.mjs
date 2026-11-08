// Builds the records of the wiki search from the Markdown files themselves. Used by the
// /search/*.json endpoints at build time and by the search test, which needs no build.
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';

import { compactBody, slugify } from './search-core.mjs';
import { devDocs, devHref, docsRoot, guideGroups, guideHref, guideOrder, guideRoot } from './docs-catalog.mjs';
import { headingId } from './rehype-docs.mjs';

/** Front matter fields, from the small YAML subset the guides use. */
export function parseFrontMatter(source) {
  const match = /^---\n([\s\S]*?)\n---\n?/.exec(source);
  if (!match) return { data: {}, body: source };
  const data = {};
  let key = null;
  for (const line of match[1].split('\n')) {
    const item = /^\s+-\s+(.*)$/.exec(line);
    if (item && key) {
      data[key].push(unquote(item[1]));
      continue;
    }
    const pair = /^([\w-]+):\s*(.*)$/.exec(line);
    if (!pair) continue;
    key = pair[1];
    const value = pair[2].trim();
    if (value === '') data[key] = [];
    else if (value.startsWith('[')) {
      data[key] = value
        .slice(1, -1)
        .split(',')
        .map((v) => unquote(v.trim()))
        .filter(Boolean);
      key = null;
    } else {
      data[key] = unquote(value);
      key = null;
    }
  }
  return { data, body: source.slice(match[0].length) };
}

function unquote(value) {
  return value.replace(/^(['"])(.*)\1$/, '$2');
}

/** Inline Markdown to plain text. */
export function plainText(markdown) {
  return markdown
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/!\[[^\]]*\]\([^)]*\)/g, ' ')
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/<[^>]+>/g, ' ')
    .replace(/[`*_>|#]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

/** Split a Markdown body into its sections at ## and ### headings. */
export function splitSections(body) {
  const lines = body.split('\n');
  const sections = [{ heading: '', depth: 1, lines: [] }];
  let inFence = false;
  for (const line of lines) {
    if (/^\s*```/.test(line)) inFence = !inFence;
    const match = !inFence && /^(#{1,3})\s+(.+?)\s*$/.exec(line);
    if (match) {
      sections.push({ heading: plainText(match[2]), depth: match[1].length, lines: [] });
    } else {
      sections[sections.length - 1].lines.push(line);
    }
  }
  return sections;
}

const guidePath = (lang, slug) => path.join(guideRoot, lang, `${slug}.md`);
const groupTitle = (slug, lang) => guideGroups.find((group) => group.slugs.includes(slug))?.title[lang] ?? '';

/** One record per section of every guide of `lang`, then per section of the developer docs. */
export function buildRecords(lang) {
  const records = [];
  let id = 0;
  for (const slug of guideOrder) {
    const source = readFileSync(guidePath(lang, slug), 'utf8');
    const { data, body } = parseFrontMatter(source);
    const seen = new Map();
    const keywords = (data.keywords ?? []).join(' ');
    let index = 0;
    for (const section of splitSections(body)) {
      const isTop = section.depth <= 1;
      if (isTop && !section.heading && !section.lines.join('').trim()) continue;
      const anchor = section.heading && !isTop ? headingId(section.heading, seen) : '';
      if (isTop && section.heading) headingId(section.heading, seen); // the h1 takes an id as well
      const text = plainText(section.lines.join('\n'));
      records.push({
        id: id++,
        title: data.title ?? slug,
        heading: isTop ? '' : section.heading,
        keywords: index === 0 ? keywords : '',
        body: compactBody(text),
        excerpt: text.slice(0, 150),
        group: groupTitle(slug, lang),
        href: `${guideHref(slug, lang)}${anchor ? `#${anchor}` : ''}`,
        dev: 0,
      });
      index += 1;
    }
  }
  for (const doc of devDocs) {
    const source = readFileSync(path.join(docsRoot, doc.file), 'utf8');
    const seen = new Map();
    for (const section of splitSections(source)) {
      if (!section.heading) continue;
      const isTop = section.depth <= 1;
      const anchor = headingId(section.heading, seen);
      const text = plainText(section.lines.join('\n'));
      records.push({
        id: id++,
        title: doc.label,
        heading: isTop ? '' : section.heading,
        keywords: '',
        body: compactBody(text, 60),
        excerpt: text.slice(0, 120),
        group: lang === 'de' ? 'Für Entwickler' : 'For developers',
        href: `${devHref(doc.slug, lang)}${isTop ? '' : `#${anchor}`}`,
        dev: doc.forDjs ? 0 : 1,
      });
    }
  }
  return records;
}

export { slugify };
export const guideFilesIn = (lang) => readdirSync(path.join(guideRoot, lang)).filter((file) => file.endsWith('.md'));
