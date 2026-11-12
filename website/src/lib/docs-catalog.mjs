// Everything about the wiki that both the Markdown plugins (astro.config.mjs), the pages and
// the check scripts need: roots, routes and the order of the guides. Plain JavaScript so the
// Astro config and node scripts load it without a build step.
import { existsSync } from 'node:fs';
import path from 'node:path';

/** The repository root: found from the working directory, because this file is also bundled. */
function findRepoRoot() {
  let dir = process.cwd();
  while (!existsSync(path.join(dir, 'Cargo.toml')) || !existsSync(path.join(dir, 'docs'))) {
    const parent = path.dirname(dir);
    if (parent === dir) throw new Error('docs-catalog: repository root not found');
    dir = parent;
  }
  return dir + path.sep;
}

export const repoRoot = findRepoRoot();
export const docsRoot = path.join(repoRoot, 'docs');
export const guideRoot = path.join(docsRoot, 'guide');

export const repoUrl = 'https://github.com/nurkert/OpenDJDecks';
export const branch = 'main';
export const githubBlob = `${repoUrl}/blob/${branch}/`;
export const githubTree = `${repoUrl}/tree/${branch}/`;

export const docLangs = ['de', 'en'];
/** German lives at the root, English under /en/. */
export const wikiHome = (lang = 'de') => (lang === 'de' ? '/wiki/' : `/${lang}/wiki/`);
export const guideHref = (slug, lang = 'de') => `${wikiHome(lang)}${slug}/`;
export const devHref = (slug, lang = 'de') => `${wikiHome(lang)}dev/${slug}/`;

/**
 * The sidebar: groups in order, guides in order. A guide missing from here fails
 * `npm run check-features`, so nothing can be written and then be unreachable.
 */
export const guideGroups = [
  { key: 'start', title: { en: 'Start here', de: 'Loslegen' }, slugs: ['getting-started', 'importing-your-library'] },
  { key: 'library', title: { en: 'Library and sets', de: 'Bibliothek und Sets' }, slugs: ['library', 'preparing-sets'] },
  {
    key: 'prepare',
    title: { en: 'Preparing tracks', de: 'Tracks vorbereiten' },
    slugs: ['cues-and-loops', 'phrases', 'beatgrid', 'editing-tracks'],
  },
  { key: 'sticks', title: { en: 'USB sticks', de: 'USB-Sticks' }, slugs: ['usb-sticks'] },
  {
    key: 'perform',
    title: { en: 'Performing', de: 'Auflegen' },
    slugs: ['perform-basics', 'mixer', 'stems', 'effects', 'sampler', 'mix-preview', 'club-mode', 'recording', 'set-safety', 'long-run-results'],
  },
  { key: 'create', title: { en: 'Making sounds', de: 'Klänge machen' }, slugs: ['create'] },
  { key: 'setup', title: { en: 'Setup', de: 'Einrichten' }, slugs: ['controllers', 'light-show', 'raspberry-pi', 'preferences'] },
  { key: 'more', title: { en: 'More', de: 'Mehr' }, slugs: ['mcp', 'troubleshooting'] },
];

export const guideOrder = guideGroups.flatMap((group) => group.slugs);

/** The developer area: the existing docs/*.md, in this order, English only. */
export const devDocs = [
  { slug: 'keyboard', file: 'keyboard.md', label: 'Keyboard shortcuts', forDjs: true },
  { slug: 'mcp', file: 'mcp.md', label: 'MCP server reference', forDjs: true },
  { slug: 'controller-mappings', file: 'controller-mappings.md', label: 'Controller mappings' },
  { slug: 'verification', file: 'verification.md', label: 'Verification status' },
];

const bySlug = new Map(devDocs.map((doc) => [doc.file.toLowerCase(), doc]));
/** The dev doc for a file name relative to docs/ (case-insensitive), or undefined. */
export const devDocForFile = (file) => bySlug.get(file.toLowerCase());
