#!/usr/bin/env node
// Neutral placeholders for the macOS install steps, until the real, anonymised screenshots
// are dropped into src/assets/install under the same names. Existing files are not touched.
import { existsSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import sharp from 'sharp';

const dir = fileURLToPath(new URL('../src/assets/install/', import.meta.url));
mkdirSync(dir, { recursive: true });
const steps = [
  ['mac-1-drag', '1', 'Open the disk image, drag to Applications'],
  ['mac-2-blocked', '2', 'macOS cannot open it: press Done'],
  ['mac-3-settings', '3', 'System Settings › Privacy and Security'],
  ['mac-4-open-anyway', '4', 'Press Open Anyway'],
  ['mac-5-confirm', '5', 'Confirm with password or Touch ID'],
];
for (const [name, number, caption] of steps) {
  const out = path.join(dir, `${name}.png`);
  if (existsSync(out)) continue;
  const svg = `<svg width="1400" height="875" xmlns="http://www.w3.org/2000/svg">
    <rect width="1400" height="875" fill="#1a1a19"/>
    <rect x="60" y="60" width="1280" height="755" fill="#151514" stroke="#33322f" stroke-width="2"/>
    <path d="M60 130h1280" stroke="#33322f" stroke-width="2"/>
    <rect x="96" y="78" width="10" height="10" fill="#33322f"/>
    <rect x="114" y="78" width="10" height="10" fill="#33322f"/>
    <path d="M300 520h300l40-56h470" stroke="#f5ce4a" stroke-width="4" fill="none"/>
    <text x="700" y="360" text-anchor="middle" fill="#e6e0d2" font-family="Barlow Semi Condensed, sans-serif" font-size="120" font-weight="600">${number}</text>
    <text x="700" y="640" text-anchor="middle" fill="#8b867b" font-family="Barlow Semi Condensed, sans-serif" font-size="36" letter-spacing="2">${caption}</text>
  </svg>`;
  await sharp(Buffer.from(svg)).png({ compressionLevel: 9 }).toFile(out);
  console.log(`wrote ${out}`);
}
