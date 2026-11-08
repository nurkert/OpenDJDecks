import { glob } from 'astro/loaders';
import { defineCollection, z } from 'astro:content';

import { devDocs } from './lib/docs-catalog.mjs';

// The DJ guides: docs/guide/<lang>/<slug>.md, with front matter.
const guide = defineCollection({
  loader: glob({ pattern: '{de,en}/*.md', base: '../docs/guide' }),
  schema: z.object({
    title: z.string(),
    description: z.string().min(20),
    keywords: z.array(z.string()).default([]),
    features: z.array(z.string()).default([]),
  }),
});

// The published developer documents, as they are, without front matter.
const dev = defineCollection({
  loader: glob({ pattern: `{${devDocs.map((doc) => doc.file).join(',')}}`, base: '../docs' }),
});

export const collections = { guide, dev };
