#!/usr/bin/env node
// Visual check: screenshots of landing, download and a wiki page at four widths in a real
// browser. Usage: node scripts/visual.mjs <outdir>. Uses the installed Chrome (CHROME_PATH).
import { createServer } from 'node:http';
import { createReadStream, existsSync, mkdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import puppeteer from 'puppeteer-core';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
const outDir = process.argv[2] ?? '/tmp/opendjdecks-shots';
mkdirSync(outDir, { recursive: true });
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.avif': 'image/avif', '.png': 'image/png', '.jpg': 'image/jpeg', '.woff2': 'font/woff2' };
const server = createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  let file = path.join(dist, decodeURIComponent(url.pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
  if (!existsSync(file)) {
    res.writeHead(404);
    res.end();
    return;
  }
  res.writeHead(200, { 'content-type': types[path.extname(file)] ?? 'application/octet-stream' });
  createReadStream(file).pipe(res);
}).listen(0, '127.0.0.1');
await new Promise((resolve) => server.once('listening', resolve));
const base = `http://127.0.0.1:${server.address().port}`;

const chromePath = process.env.CHROME_PATH ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const browser = await puppeteer.launch({ executablePath: chromePath, headless: true, args: ['--no-sandbox'] });
const widths = [390, 768, 1280, 1920];
const heights = { 390: 844, 768: 1024, 1280: 800, 1920: 1080 };
const pages = [
  ['home', '/'],
  ['download', '/en/download/'],
  ['wiki', '/wiki/editing-tracks/'],
];
const report = [];
for (const [name, route] of pages) {
  for (const width of widths) {
    const page = await browser.newPage();
    await page.setViewport({ width, height: heights[width], deviceScaleFactor: 1 });
    await page.goto(base + route, { waitUntil: 'networkidle0' });
    // Light up every section: scroll through, so lazy pictures load and scroll scenes run.
    const scrollWidth = await page.evaluate(async () => {
      const total = document.documentElement.scrollHeight;
      for (let y = 0; y < total; y += 500) {
        window.scrollTo(0, y);
        await new Promise((r) => setTimeout(r, 60));
      }
      window.scrollTo(0, 0);
      await new Promise((r) => setTimeout(r, 300));
      return document.documentElement.scrollWidth;
    });
    if (scrollWidth > width) report.push(`${name}@${width}: horizontal overflow ${scrollWidth}px`);
    await page.screenshot({ path: path.join(outDir, `${name}-${width}-top.png`) });
    await page.screenshot({ path: path.join(outDir, `${name}-${width}-full.png`), fullPage: true });
    if (name === 'home' && width >= 1280) {
      for (const [label, at] of [['mid', 0.32], ['end', 0.62]]) {
        await page.evaluate((f) => window.scrollTo(0, f * document.querySelector('[data-scene]').offsetHeight), at);
        await new Promise((r) => setTimeout(r, 350));
        await page.screenshot({ path: path.join(outDir, `${name}-${width}-scroll-${label}.png`) });
      }
    }
    await page.close();
  }
}
await browser.close();
server.close();
console.log(report.length ? report.join('\n') : 'No horizontal overflow at any width.');
