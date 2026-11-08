import de from './locales/de';
import en from './locales/en';

export const langs = ['de', 'en'] as const;
export type Lang = (typeof langs)[number];
export const defaultLang: Lang = 'de';
export const languages: Record<Lang, { short: string; label: string; og: string }> = {
  de: { short: 'DE', label: 'Deutsch', og: 'de_DE' },
  en: { short: 'EN', label: 'English', og: 'en_US' },
};

export const repoUrl = 'https://github.com/nurkert/OpenDJDecks';
export const releasesUrl = `${repoUrl}/releases`;

export const ui = { de, en };
export type Ui = typeof en;

export const pages = {
  home: { de: '', en: '' },
  download: { de: 'download', en: 'download' },
  imprint: { de: 'impressum', en: 'imprint' },
  privacy: { de: 'datenschutz', en: 'privacy' },
} as const;
export type Page = keyof typeof pages;

/** The route parameter of [...route].astro: `undefined` is the site root. */
export function routeParam(page: Page, lang: Lang): string | undefined {
  const segment = pages[page][lang];
  const parts = lang === defaultLang ? [segment] : [lang, segment];
  return parts.filter(Boolean).join('/') || undefined;
}

export function path(page: Page, lang: Lang): string {
  const param = routeParam(page, lang);
  return param ? `/${param}/` : '/';
}

export function alternates(page: Page): { lang: Lang; href: string }[] {
  return langs.map((lang) => ({ lang, href: path(page, lang) }));
}
