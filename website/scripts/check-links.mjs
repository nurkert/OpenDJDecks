#!/usr/bin/env node
// Dead internal links and anchors in the built site, and hreflang pairs on every page that
// has a language version. Run after `astro build`.
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
if (!existsSync(dist)) {
  console.error('dist/ not found: run the build first.');
  process.exit(1);
}

const pages = new Map();
const walk = (dir) => {
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    if (statSync(full).isDirectory()) walk(full);
    else if (name === 'index.html') {
      const route = `/${path.relative(dist, path.dirname(full)).split(path.sep).join('/')}/`.replace(/^\/\.?\//, '/');
      pages.set(route, readFileSync(full, 'utf8'));
    }
  }
};
walk(dist);

const ids = new Map();
for (const [route, html] of pages) {
  ids.set(route, new Set([...html.matchAll(/\sid="([^"]+)"/g)].map((m) => m[1])));
}

const exists = (urlPath) => {
  const clean = decodeURI(urlPath);
  if (pages.has(clean)) return true;
  const file = path.join(dist, clean);
  return existsSync(file) && statSync(file).isFile();
};

const problems = [];
for (const [route, html] of pages) {
  const refs = [...html.matchAll(/\s(?:href|src)="([^"#][^"]*|#[^"]+)"/g)].map((m) => m[1]);
  for (const ref of refs) {
    if (/^(?:https?:|mailto:|data:|\/\/)/.test(ref)) continue;
    const [pathPart, hash] = ref.replace(/&amp;/g, '&').split('#');
    const target = pathPart || route;
    const resolved = target.startsWith('/') ? target : new URL(target, `https://x${route}`).pathname;
    if (!exists(resolved) && !exists(`${resolved.replace(/\/?$/, '/')}`)) {
      problems.push(`${route}: dead link ${ref}`);
      continue;
    }
    const key = pages.has(resolved) ? resolved : `${resolved.replace(/\/?$/, '/')}`;
    if (hash && pages.has(key) && !ids.get(key).has(decodeURIComponent(hash))) {
      problems.push(`${route}: dead anchor ${ref}`);
    }
  }
  const isWiki = route.startsWith('/wiki/') || route.startsWith('/en/wiki/');
  if (!html.includes('rel="canonical"')) problems.push(`${route}: no canonical`);
  const needsHreflang = !route.includes('/dev/') && (isWiki || !/404/.test(route));
  if (needsHreflang && !html.includes('hreflang="de"')) problems.push(`${route}: no hreflang pair`);
}

if (problems.length) {
  console.error(`Link check FAILED (${problems.length}):\n  ${problems.join('\n  ')}`);
  process.exit(1);
}
if (pages.size === 0) {
  console.error('Link check FAILED: no pages found, was the site built?');
  process.exit(1);
}
console.log(`Link check OK: ${pages.size} pages.`);
