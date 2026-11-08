// Reading the wiki content: titles and entries from the content collections.
import { getCollection, getEntry, type CollectionEntry } from 'astro:content';

import { devDocs, devHref, docLangs, guideHref, guideOrder } from './docs-catalog.mjs';

export type Lang = 'de' | 'en';
export type GuideEntry = CollectionEntry<'guide'>;

export const guideEntry = (slug: string, lang: Lang) => getEntry('guide', `${lang}/${slug}`);

export async function guideTitle(slug: string, lang: Lang): Promise<string> {
  const entry = await guideEntry(slug, lang);
  return entry?.data.title ?? slug;
}

export async function allGuides() {
  return getCollection('guide');
}

export async function devEntries() {
  const all = await getCollection('dev');
  return devDocs.map((doc) => ({ doc, entry: all.find((entry) => entry.filePath?.endsWith(`/${doc.file}`))! }));
}

/** Language versions of a guide page, for hreflang and the language switch. */
export const guideAlternates = (slug: string) => docLangs.map((lang) => ({ lang: lang as Lang, href: guideHref(slug, lang) }));
export const devAlternates = (slug: string) => docLangs.map((lang) => ({ lang: lang as Lang, href: devHref(slug, lang) }));

/** Previous and next guide, in sidebar order across the groups. */
export function guideNeighbours(slug: string) {
  const index = guideOrder.indexOf(slug);
  return { previous: guideOrder[index - 1] as string | undefined, next: guideOrder[index + 1] as string | undefined };
}

export function devNeighbours(slug: string) {
  const index = devDocs.findIndex((doc) => doc.slug === slug);
  return { previous: devDocs[index - 1], next: devDocs[index + 1] };
}
