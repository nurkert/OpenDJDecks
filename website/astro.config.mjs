import { unified } from '@astrojs/markdown-remark';
import { defineConfig } from 'astro/config';

import { repoRoot } from './src/lib/docs-catalog.mjs';
import rehypeDocs from './src/lib/rehype-docs.mjs';
import remarkDocs from './src/lib/remark-docs.mjs';

export default defineConfig({
  site: 'https://opendjdecks.de',
  trailingSlash: 'always',
  build: {
    // CSS and JS stay in files so the Content-Security-Policy needs no inline scripts.
    inlineStylesheets: 'never',
  },
  // Pictures in the Markdown guides get the same treatment as the ones on the pages:
  // WebP and a srcset, with their dimensions in the HTML.
  image: { layout: 'constrained', responsiveStyles: false },
  markdown: {
    // Typographic replacements stay off: buttons, keys and commands appear exactly as in the app.
    processor: unified({
      smartypants: false,
      remarkPlugins: [remarkDocs],
      rehypePlugins: [rehypeDocs],
    }),
    syntaxHighlight: 'shiki',
    shikiConfig: { theme: 'css-variables' },
  },
  vite: {
    build: { assetsInlineLimit: 0 },
    // The guides and their pictures live in docs/, outside website/.
    server: { fs: { allow: [repoRoot] } },
  },
});
