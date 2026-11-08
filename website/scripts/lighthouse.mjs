#!/usr/bin/env node
// Lighthouse against the built site: landing, download and one wiki page, on mobile and
// desktop presets. Below 95 in any category the build fails. Uses the Chrome already on the
// machine (CHROME_PATH), so nothing is downloaded. Scores can wobble by a point on a noisy
// CI runner, so a failing page is retried once before it counts.
import { createServer } from 'node:http';
import { createReadStream, existsSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import lighthouse from 'lighthouse';
import * as chromeLauncher from 'chrome-launcher';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
const threshold = Number(process.env.LH_MIN ?? 95);
const pages = ['/', '/en/download/', '/wiki/editing-tracks/'];
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.avif': 'image/avif', '.png': 'image/png', '.jpg': 'image/jpeg', '.woff2': 'font/woff2', '.xml': 'application/xml', '.txt': 'text/plain', '.webm': 'video/webm', '.mp4': 'video/mp4' };

const server = createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  let file = path.join(dist, decodeURIComponent(url.pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
  if (!existsSync(file)) {
    res.statusCode = 404;
    res.end('not found');
    return;
  }
  const headers = { 'content-type': types[path.extname(file)] ?? 'application/octet-stream' };
  if (/\/_astro\//.test(file)) headers['cache-control'] = 'public, max-age=31536000, immutable';
  res.writeHead(200, headers);
  createReadStream(file).pipe(res);
}).listen(0, '127.0.0.1');
await new Promise((resolve) => server.once('listening', resolve));
const base = `http://127.0.0.1:${server.address().port}`;

const chrome = await chromeLauncher.launch({
  chromePath: process.env.CHROME_PATH || undefined,
  chromeFlags: ['--headless=new', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage'],
});

const categories = ['performance', 'accessibility', 'best-practices', 'seo'];
const desktopFormFactor = {
  formFactor: 'desktop',
  throttling: { rttMs: 40, throughputKbps: 10240, cpuSlowdownMultiplier: 1, requestLatencyMs: 0, downloadThroughputKbps: 0, uploadThroughputKbps: 0 },
  screenEmulation: { mobile: false, width: 1350, height: 940, deviceScaleFactor: 1, disabled: false },
  emulatedUserAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36',
};

async function audit(page, formFactor) {
  const settings = {
    port: chrome.port,
    output: 'json',
    logLevel: 'error',
    onlyCategories: categories,
  };
  const config = {
    extends: 'lighthouse:default',
    settings: formFactor === 'desktop' ? desktopFormFactor : {},
  };
  const result = await lighthouse(`${base}${page}`, settings, config);
  const scores = Object.fromEntries(categories.map((c) => [c, Math.round((result.lhr.categories[c].score ?? 0) * 100)]));
  const failures = Object.values(result.lhr.audits)
    .filter((a) => a.score !== null && a.score < 1 && a.scoreDisplayMode === 'binary')
    .map((a) => a.id);
  return { scores, failures };
}

const results = [];
let failed = false;
try {
  for (const page of pages) {
    for (const formFactor of ['mobile', 'desktop']) {
      let run = await audit(page, formFactor);
      let below = categories.filter((c) => run.scores[c] < threshold);
      if (below.length) {
        run = await audit(page, formFactor);
        below = categories.filter((c) => run.scores[c] < threshold);
      }
      console.log(`${page.padEnd(26)} ${formFactor.padEnd(8)} ${categories.map((c) => `${c.slice(0, 4)} ${run.scores[c]}`).join('  ')}`);
      if (below.length) {
        failed = true;
        console.log(`  below ${threshold}: ${below.join(', ')}; failing audits: ${run.failures.join(', ') || 'see report'}`);
      }
      results.push({ page, formFactor, ...run.scores });
    }
  }
} finally {
  await chrome.kill();
  server.close();
}
if (failed) {
  console.error(`Lighthouse FAILED: some category below ${threshold}.`);
  process.exit(1);
}
console.log(`Lighthouse OK: all categories ≥ ${threshold}.`);
