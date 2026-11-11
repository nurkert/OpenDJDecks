//! Fuzzy search over the browser's lists ([26]).
//!
//! Every word of the query has to be found somewhere in the track: in the
//! title, artist or album as a whole word, a word start, a part of a word
//! or a word with one typo (two in long words); genre, label, remixer and
//! comment count when they hold the word as it is typed. Each hit scores by
//! how good it is and where it was found (title before artist before
//! album), so the best matches come first.

use od_library::TrackRecord;

/// How the query is held: lower-case words with accents folded.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Query {
    words: Vec<String>,
    chars: Vec<Vec<char>>,
    masks: Vec<u64>,
    whole: String,
}

/// The letters of a word as a bit set (a–z, 0–9, the rest shared), for a
/// quick lower bound of the typos between two words: every letter of the
/// query the word lacks costs at least one edit.
fn mask(chars: &[char]) -> u64 {
    chars.iter().fold(0u64, |m, c| {
        m | 1 << match c {
            'a'..='z' => *c as u32 - 'a' as u32,
            '0'..='9' => 26 + (*c as u32 - '0' as u32),
            _ => 63,
        }
    })
}

impl Query {
    pub fn new(q: &str) -> Self {
        let whole = fold(q.trim());
        let words: Vec<String> = words(&whole).map(str::to_string).collect();
        let chars: Vec<Vec<char>> = words.iter().map(|w| w.chars().collect()).collect();
        let masks = chars.iter().map(|c| mask(c)).collect();
        Self { words, chars, masks, whole }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

/// Lower case, common accents and umlauts folded to their base letter.
pub(crate) fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => 'a',
            'ç' | 'ć' | 'č' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
            'ñ' | 'ń' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'ū' => 'u',
            'ý' | 'ÿ' => 'y',
            'ś' | 'š' | 'ß' => 's',
            'ž' | 'ź' | 'ż' => 'z',
            'ł' => 'l',
            c => c,
        })
        .collect()
}

fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty())
}

/// Optimal string alignment distance (a swap of two neighbours counts
/// one), stopping early above `max`.
fn distance(a: &[char], b: &[char], max: usize) -> usize {
    if a.len().abs_diff(b.len()) > max {
        return max + 1;
    }
    let (n, m) = (a.len(), b.len());
    // Rows on the stack for ordinary words ([63]: no allocation per pair).
    const ROW: usize = 48;
    if m + 1 > ROW {
        return distance_long(a, b, max);
    }
    let mut prev2 = [0usize; ROW];
    let mut prev = [0usize; ROW];
    for (j, v) in prev.iter_mut().enumerate().take(m + 1) {
        *v = j;
    }
    let mut cur = [0usize; ROW];
    for i in 1..=n {
        cur[0] = i;
        let mut row_min = cur[0];
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(prev2[j - 2] + 1);
            }
            cur[j] = v;
            row_min = row_min.min(v);
        }
        if row_min > max {
            return max + 1;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

/// [`distance`] for words longer than its stack rows.
fn distance_long(a: &[char], b: &[char], max: usize) -> usize {
    let (n, m) = (a.len(), b.len());
    let mut prev2 = vec![0usize; m + 1];
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    for i in 1..=n {
        cur[0] = i;
        let mut row_min = cur[0];
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(prev2[j - 2] + 1);
            }
            cur[j] = v;
            row_min = row_min.min(v);
        }
        if row_min > max {
            return max + 1;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

/// Typos allowed in a query word of this length.
fn typos(len: usize) -> usize {
    match len {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

/// A field folded once ([63]): its text and its words with their chars.
#[derive(Clone, Debug, Default)]
pub(crate) struct Field {
    text: String,
    words: Vec<(String, Vec<char>, u64)>,
}

impl Field {
    fn new(raw: &str) -> Self {
        let text = fold(raw);
        let words = words(&text)
            .map(|w| {
                let c: Vec<char> = w.chars().collect();
                let m = mask(&c);
                (w.to_string(), c, m)
            })
            .collect();
        Self { text, words }
    }
}

/// A track's searchable text, folded once and kept while only the search
/// changes ([63]).
#[derive(Clone, Debug, Default)]
pub(crate) struct Folded {
    title: Field,
    artist: Field,
    album: Field,
    rest: String,
}

impl Folded {
    pub fn of(t: &TrackRecord) -> Self {
        Self {
            title: Field::new(&t.tags.title),
            artist: Field::new(&t.tags.artist),
            album: Field::new(&t.tags.album),
            rest: fold(&format!(
                "{} {} {} {}",
                t.tags.genre, t.tags.label, t.tags.remixer, t.tags.comment
            )),
        }
    }
}

/// How well one query word matches one field (0 = not at all).
fn word_score(q: &str, qc: &[char], qm: u64, field: &Field) -> u32 {
    let max = typos(qc.len());
    let mut best = 0;
    for (w, wc, wm) in &field.words {
        let w = w.as_str();
        let s = if w == q {
            100
        } else if w.starts_with(q) {
            80
        } else if w.contains(q) && qc.len() >= 3 {
            55
        } else if max > 0 && (qm & !wm).count_ones() as usize <= max {
            // The whole word, or its start as long as the query (a typo in
            // a word still being typed).
            let whole = distance(qc, wc, max);
            let start = if wc.len() > qc.len() {
                distance(qc, &wc[..qc.len()], max)
            } else {
                max + 1
            };
            match whole.min(start) {
                d if d <= max => 45 - 10 * d as u32,
                _ => 0,
            }
        } else {
            0
        };
        best = best.max(s);
        if best == 100 {
            break;
        }
    }
    best
}

/// The track's score for `q` (`None` when a query word is found nowhere).
pub(crate) fn score(q: &Query, t: &TrackRecord) -> Option<u32> {
    if q.is_empty() {
        return Some(0);
    }
    score_folded(q, &Folded::of(t))
}

/// [`score`] on text folded beforehand.
pub(crate) fn score_folded(q: &Query, f: &Folded) -> Option<u32> {
    if q.is_empty() {
        return Some(0);
    }
    let mut total = 0;
    for ((w, wc), qm) in q.words.iter().zip(&q.chars).zip(&q.masks) {
        // Best place first: a title hit outweighs anything else.
        let t = word_score(w, wc, *qm, &f.title) * 3;
        let s = if t >= 300 {
            t
        } else {
            t.max(word_score(w, wc, *qm, &f.artist) * 2)
                .max(word_score(w, wc, *qm, &f.album))
                .max(if f.rest.contains(w.as_str()) { 40 } else { 0 })
        };
        if s == 0 {
            return None;
        }
        total += s;
    }
    // The query as typed, in one piece.
    if q.words.len() > 1 || q.whole.len() >= 3 {
        if f.title.text.starts_with(&q.whole) {
            total += 150;
        } else if f.title.text.contains(&q.whole) || f.artist.text.contains(&q.whole) {
            total += 60;
        }
    }
    Some(total)
}

/// Indices of the tracks that match `q` with their scores, in list order
/// ([63]: over text folded once).
pub(crate) fn matches(q: &Query, folded: &[Folded]) -> Vec<(usize, u32)> {
    folded
        .iter()
        .enumerate()
        .filter_map(|(i, f)| score_folded(q, f).map(|s| (i, s)))
        .collect()
}

/// Keeps the tracks that match `q`, best first (a stable sort: equal
/// scores keep the order they came in).
pub(crate) fn filter_rank(rows: &mut Vec<TrackRecord>, q: &Query, rank: bool) {
    let mut scored: Vec<(u32, TrackRecord)> = rows
        .drain(..)
        .filter_map(|t| score(q, &t).map(|s| (s, t)))
        .collect();
    if rank {
        scored.sort_by(|a, b| b.0.cmp(&a.0));
    }
    rows.extend(scored.into_iter().map(|(_, t)| t));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(title: &str, artist: &str, album: &str) -> TrackRecord {
        let mut t: TrackRecord = serde_json::from_value(serde_json::json!({
            "id": 1, "path": "", "content_hash": "", "file_size": 0, "format": null, "duration": 0.0,
            "sample_rate": 44100, "channels": 2, "bit_rate": null, "bits_per_sample": null,
            "tags": od_core::TrackTags::default(), "date_added": "", "play_count": 0, "last_played": null,
            "missing": false, "bpm": null, "key": null, "analysis_version": 0
        }))
        .unwrap();
        t.tags.title = title.into();
        t.tags.artist = artist.into();
        t.tags.album = album.into();
        t
    }

    fn titles(rows: &[TrackRecord]) -> Vec<&str> {
        rows.iter().map(|t| t.tags.title.as_str()).collect()
    }

    #[test]
    fn typos_are_forgiven_by_word_length() {
        let q = |s| Query::new(s);
        let t = track("Midnight Express", "Nova Lines", "Night Works");
        assert!(score(&q("midnight"), &t).is_some());
        assert!(score(&q("midnigth"), &t).is_some(), "swapped letters");
        assert!(score(&q("midnght"), &t).is_some(), "a missing letter");
        assert!(score(&q("expres"), &t).is_some(), "a word start");
        assert!(score(&q("nvoa"), &t).is_some(), "a swap in a short word");
        assert!(score(&q("nxa"), &t).is_none(), "three letters must match");
        assert!(score(&q("mdnght"), &t).is_none(), "two typos in a short word");
        assert!(score(&q("ëxpréss"), &t).is_some(), "accents fold");
    }

    #[test]
    fn every_word_has_to_match_somewhere() {
        let t = track("Afterglow", "Mira Vale", "Summer");
        assert!(score(&Query::new("mira afterglow"), &t).is_some(), "across fields");
        assert!(score(&Query::new("vale summer"), &t).is_some(), "artist and album");
        assert!(score(&Query::new("mira techno"), &t).is_none());
        let mut g = t.clone();
        g.tags.genre = "Deep House".into();
        assert!(score(&Query::new("house"), &g).is_some(), "genre as typed");
    }

    #[test]
    fn good_matches_rank_first() {
        let mut rows = vec![
            track("Late Shift", "Night Crew", "Nightlife"),
            track("Nightdrive", "Mira Vale", ""),
            track("Night Drive", "Nova Lines", ""),
            track("Kninght", "Someone", ""),
            track("Unrelated", "Other", ""),
        ];
        filter_rank(&mut rows, &Query::new("night drive"), true);
        assert_eq!(titles(&rows)[0], "Night Drive", "{:?}", titles(&rows));
        assert!(!titles(&rows).contains(&"Unrelated"));

        let mut rows = vec![
            track("Album Song", "X", "Night"),
            track("Artist Song", "Night", ""),
            track("Night", "X", ""),
        ];
        filter_rank(&mut rows, &Query::new("night"), true);
        assert_eq!(titles(&rows), ["Night", "Artist Song", "Album Song"], "title, artist, album");
    }

    #[test]
    fn unranked_keeps_the_list_order() {
        let mut rows = vec![track("B Night", "", ""), track("Night", "", "")];
        filter_rank(&mut rows, &Query::new("night"), false);
        assert_eq!(titles(&rows), ["B Night", "Night"]);
    }

    #[test]
    fn distance_counts_swaps_once() {
        let c = |s: &str| s.chars().collect::<Vec<_>>();
        assert_eq!(distance(&c("abcd"), &c("abdc"), 2), 1);
        assert_eq!(distance(&c("abcd"), &c("abcd"), 2), 0);
        assert_eq!(distance(&c("abcd"), &c("xyzw"), 2), 3, "stops above max");
    }
}
