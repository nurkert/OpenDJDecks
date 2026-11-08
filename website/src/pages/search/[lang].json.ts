// The wiki search index, one file per language, loaded on the first focus of the search field.
import type { APIRoute } from 'astro';

import { buildRecords } from '../../lib/search-index.mjs';
import { docLangs } from '../../lib/docs-catalog.mjs';

export function getStaticPaths() {
  return docLangs.map((lang) => ({ params: { lang } }));
}

export const GET: APIRoute = ({ params }) =>
  new Response(JSON.stringify(buildRecords(params.lang as string)), { headers: { 'Content-Type': 'application/json' } });
