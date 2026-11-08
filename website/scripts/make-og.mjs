#!/usr/bin/env node
// The social preview: public/og.jpg, 1200×630, from the app's banner.
import { fileURLToPath } from 'node:url';

import sharp from 'sharp';

const banner = fileURLToPath(new URL('../../docs/assets/banner.png', import.meta.url));
const out = fileURLToPath(new URL('../public/og.jpg', import.meta.url));
await sharp({ create: { width: 1200, height: 630, channels: 3, background: '#151514' } })
  .composite([{ input: await sharp(banner).resize(1200, 400, { fit: 'contain', background: '#151514' }).toBuffer(), top: 115, left: 0 }])
  .jpeg({ quality: 85, mozjpeg: true })
  .toFile(out);
console.log(`wrote ${out}`);
