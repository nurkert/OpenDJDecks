// The sitemap with hreflang pairs for every page in both languages.
import type { APIRoute } from 'astro';

import { alternates, defaultLang, pages, path, type Page } from '../i18n/ui';
import { devDocs, devHref, docLangs, guideHref, guideOrder, wikiHome } from '../lib/docs-catalog.mjs';

export const GET: APIRoute = ({ site }) => {
  const url = (href: string) => new URL(href, site).toString();
  const pair = (versions: { lang: string; href: string }[], fallback: string) => {
    const links = [
      ...versions.map((v) => `<xhtml:link rel="alternate" hreflang="${v.lang}" href="${url(v.href)}"/>`),
      `<xhtml:link rel="alternate" hreflang="x-default" href="${url(fallback)}"/>`,
    ].join('');
    return versions.map((v) => `<url><loc>${url(v.href)}</loc>${links}</url>`);
  };
  const entries = (Object.keys(pages) as Page[]).flatMap((page) => pair(alternates(page), path(page, defaultLang)));
  entries.push(...pair(docLangs.map((lang) => ({ lang, href: wikiHome(lang) })), wikiHome('de')));
  for (const slug of guideOrder) {
    entries.push(...pair(docLangs.map((lang) => ({ lang, href: guideHref(slug, lang) })), guideHref(slug, 'de')));
  }
  // Developer docs: only the canonical English URL is listed.
  for (const doc of devDocs) entries.push(`<url><loc>${url(devHref(doc.slug, 'en'))}</loc></url>`);
  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${entries.join('\n')}
</urlset>`;
  return new Response(body, { headers: { 'Content-Type': 'application/xml; charset=utf-8' } });
};
