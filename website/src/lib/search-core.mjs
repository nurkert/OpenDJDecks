// What the build (index), the browser (query) and the test (`scripts/search.test.mjs`) share,
// so the test exercises exactly what visitors get.

/** Normalise a word for matching: lower case, German umlauts folded, no accents. */
export function foldTerm(term) {
  return term
    .toLowerCase()
    .replace(/ä/g, 'ae')
    .replace(/ö/g, 'oe')
    .replace(/ü/g, 'ue')
    .replace(/ß/g, 'ss')
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '');
}

export const tokenize = (text) => text.split(/[\s\-–—/·:,.;!?()[\]"'„“”‚‘’`<>=+*#|~]+/u).filter(Boolean);

/** MiniSearch field weights. Keywords carry the words DJs actually type. */
export const searchFields = ['title', 'heading', 'keywords', 'body'];

export function miniSearchOptions() {
  return {
    fields: searchFields,
    storeFields: ['title', 'heading', 'href', 'group', 'excerpt', 'dev'],
    processTerm: (term) => {
      const folded = foldTerm(term);
      return folded.length > 1 ? folded : null;
    },
    tokenize,
    searchOptions: {
      boost: { title: 8, keywords: 6, heading: 3, body: 1 },
      prefix: (term) => term.length >= 3,
      // A typo of one letter in short words, two in long ones: "beatgrit", "zuschneidn".
      fuzzy: (term) => (term.length >= 4 ? (term.length >= 9 ? 0.25 : 0.22) : false),
      // Any word may match; scores add up, so pages matching more of the words come first.
      combineWith: 'OR',
      // Guides beat the developer reference when both match.
      boostDocument: (_id, _term, stored) => (stored?.dev ? 0.25 : 1),
    },
  };
}

/**
 * Best section per guide first: the page a visitor is looking for must not be pushed down by
 * three other sections of a longer document. `results` are MiniSearch results, best first.
 */
export function dedupeByPage(results, limit = 12) {
  const seen = new Set();
  const out = [];
  for (const result of results) {
    const page = result.href.split('#')[0];
    if (seen.has(page)) continue;
    seen.add(page);
    out.push(result);
    if (out.length >= limit) break;
  }
  return out;
}

/** A heading's id: the same function the rehype plugin and the search index use. */
export function slugify(value) {
  return foldTerm(value)
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, ' ')
    .trim()
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-');
}

/** Stop words we can drop from the body: shrinks the index without hurting searches. */
const stop = new Set(
  `the a an and or of to in on at is it for with as by be this that you your are from can then when will not
  die der das ein eine einen einem einer und oder von zu im in am an auf ist es für mit als wie bei dass du dein
  deine deinen wird werden nicht sich dann wenn auch aus den dem des hier nur noch über so sie er ihr`
    .split(/\s+/)
    .filter(Boolean),
);

/** Compact body text: unique, non-stop words only, capped. Word order does not matter here. */
export function compactBody(text, maxWords = 160) {
  const seen = new Set();
  const words = [];
  for (const raw of tokenize(text)) {
    const folded = foldTerm(raw);
    if (folded.length < 2 || stop.has(folded) || seen.has(folded)) continue;
    seen.add(folded);
    words.push(raw);
    if (words.length >= maxWords) break;
  }
  return words.join(' ');
}
