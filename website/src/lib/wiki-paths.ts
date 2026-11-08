import { devEntries, type Lang } from './docs';
import { guideOrder } from './docs-catalog.mjs';
import { getEntry } from 'astro:content';

/** Static routes of the wiki in one language: every guide and every developer document. */
export async function wikiPaths(lang: Lang) {
  const guides = await Promise.all(
    guideOrder.map(async (slug) => ({
      params: { slug },
      props: { kind: 'guide' as const, slug, entry: await getEntry('guide', `${lang}/${slug}`) },
    })),
  );
  const devs = (await devEntries()).map(({ doc, entry }) => ({
    params: { slug: `dev/${doc.slug}` },
    props: { kind: 'dev' as const, slug: doc.slug, entry },
  }));
  return [...guides, ...devs];
}
