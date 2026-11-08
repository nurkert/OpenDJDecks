//! AUTO SORT: the collection classified offline from its own analysis, in
//! five groups of lists — tempo bands, energy, mood, character (the balance
//! of the waveform bands) and genre families.
//!
//! Classification is pure ([`classify`]); [`Library::auto_profiles`] reads
//! the analysis data it needs beyond the track rows (energy curve, phrase
//! mood, band balance), skipping tracks whose profile is already known.

use crate::{Library, Result, blob};
use od_core::{Mode, MusicalKey, PhraseMood, TrackId};
use std::collections::HashMap;

/// What classification needs from a track's analysis beyond its row.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrackProfile {
    /// Mean of the 0..=1 energy curve.
    pub energy: Option<f32>,
    /// Phrase mood of the analysis.
    pub mood: Option<PhraseMood>,
    /// Shares of the low, mid and high band in the waveform (sum 1).
    pub bands: Option<[f32; 3]>,
}

/// One group of the AUTO SORT tree, in tree order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AutoGroup {
    Tempo,
    Energy,
    Mood,
    Character,
    Genre,
}

impl AutoGroup {
    pub const ALL: [AutoGroup; 5] = [
        AutoGroup::Tempo,
        AutoGroup::Energy,
        AutoGroup::Mood,
        AutoGroup::Character,
        AutoGroup::Genre,
    ];

    /// Stable id ("tempo", …), the prefix of the list keys.
    pub fn id(self) -> &'static str {
        match self {
            AutoGroup::Tempo => "tempo",
            AutoGroup::Energy => "energy",
            AutoGroup::Mood => "mood",
            AutoGroup::Character => "character",
            AutoGroup::Genre => "genre",
        }
    }

    /// Name in the tree (English; the UI translates it).
    pub fn title(self) -> &'static str {
        match self {
            AutoGroup::Tempo => "Tempo",
            AutoGroup::Energy => "Energy",
            AutoGroup::Mood => "Mood",
            AutoGroup::Character => "Character",
            AutoGroup::Genre => "Genre",
        }
    }
}

/// One list of the AUTO SORT tree.
#[derive(Clone, Debug, PartialEq)]
pub struct AutoList {
    pub group: AutoGroup,
    /// Stable key, e.g. "tempo:124-130", "energy:high", "genre:house".
    pub key: String,
    /// Name in the tree (English for the fixed lists; the UI translates it).
    pub name: String,
    pub tracks: Vec<TrackId>,
}

/// What [`classify`] reads of one track.
#[derive(Clone, Debug)]
pub struct AutoInput {
    pub id: TrackId,
    pub bpm: Option<f64>,
    pub key: Option<MusicalKey>,
    pub genre: String,
    pub profile: TrackProfile,
}

/// Genre-agnostic tempo bands: [from, to) BPM, key, name.
pub const TEMPO_BANDS: [(f64, f64, &str, &str); 7] = [
    (0.0, 100.0, "0-100", "< 100 BPM"),
    (100.0, 115.0, "100-115", "100–115 BPM"),
    (115.0, 124.0, "115-124", "115–124 BPM"),
    (124.0, 130.0, "124-130", "124–130 BPM"),
    (130.0, 140.0, "130-140", "130–140 BPM"),
    (140.0, 160.0, "140-160", "140–160 BPM"),
    (160.0, f64::INFINITY, "160-", "160+ BPM"),
];

/// Genre families: name and the words that put a genre tag in it (first
/// family that matches wins, so "Afro House" is House and "Drum & Bass" is
/// not Dubstep & Bass).
const FAMILIES: &[(&str, &[&str])] = &[
    (
        "Drum & Bass",
        &[
            "drum & bass",
            "drum and bass",
            "drum'n'bass",
            "drum n bass",
            "drumnbass",
            "dnb",
            "d&b",
            "jungle",
            "neurofunk",
        ],
    ),
    (
        "Hard Dance",
        &[
            "hardstyle",
            "hardcore",
            "hard dance",
            "gabber",
            "hard trance",
        ],
    ),
    ("Garage", &["garage", "2-step", "2step", "bassline"]),
    ("House", &["house"]),
    ("Techno", &["techno", "minimal"]),
    ("Trance", &["trance", "psy"]),
    (
        "Dubstep & Bass",
        &["dubstep", "riddim", "bass music", "future bass"],
    ),
    (
        "Hip-Hop & R&B",
        &[
            "hip hop", "hip-hop", "hiphop", "rap", "trap", "r&b", "rnb", "grime", "drill",
        ],
    ),
    ("Disco & Funk", &["disco", "funk", "soul", "boogie"]),
    (
        "Electro & Breaks",
        &["electro", "breaks", "breakbeat", "big beat"],
    ),
    ("Afro & Amapiano", &["afro", "amapiano", "kuduro"]),
    (
        "Latin",
        &[
            "latin",
            "reggaeton",
            "dembow",
            "moombahton",
            "salsa",
            "cumbia",
            "baile",
        ],
    ),
    (
        "Downtempo & Ambient",
        &[
            "downtempo",
            "ambient",
            "chill",
            "lounge",
            "lofi",
            "lo-fi",
            "trip hop",
            "trip-hop",
        ],
    ),
    ("Pop", &["pop"]),
    ("Rock", &["rock", "indie", "metal", "punk"]),
];

/// Lists of a genre tag outside every family when that many tracks carry it.
const OWN_GENRE_MIN: usize = 2;

/// Whether `genre` (lower case) carries `word`: short words must stand on
/// their own ("rap" is not in "trapeze"), longer ones may be part of a word
/// ("deephouse").
fn has_word(genre: &str, word: &str) -> bool {
    if word.chars().count() >= 5 {
        return genre.contains(word);
    }
    let alnum = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    genre.match_indices(word).any(|(i, _)| {
        !alnum(genre[..i].chars().next_back()) && !alnum(genre[i + word.len()..].chars().next())
    })
}

/// The family of a genre tag, if one fits.
pub fn genre_family(genre: &str) -> Option<&'static str> {
    let g = genre.trim().to_lowercase();
    if g.is_empty() {
        return None;
    }
    FAMILIES
        .iter()
        .find(|(_, words)| words.iter().any(|w| has_word(&g, w)))
        .map(|(name, _)| *name)
}

/// The tempo band index of `bpm`.
pub fn tempo_band(bpm: f64) -> Option<usize> {
    (bpm.is_finite() && bpm > 0.0)
        .then(|| {
            TEMPO_BANDS
                .iter()
                .position(|(lo, hi, ..)| bpm >= *lo && bpm < *hi)
        })
        .flatten()
}

/// Mood from the key's mode and the phrase mood: (key, name).
pub fn mood(key: MusicalKey, phrase: Option<PhraseMood>) -> (&'static str, &'static str) {
    let high = phrase == Some(PhraseMood::High);
    match (key.mode, high) {
        (Mode::Major, true) => ("euphoric", "Euphoric"),
        (Mode::Major, false) => ("warm", "Warm"),
        (Mode::Minor, true) => ("driving", "Driving"),
        (Mode::Minor, false) => ("dark", "Dark"),
    }
}

/// Splits `values` (id, value) into thirds by rank (ties by id, so the
/// split is stable): 0 = lowest third, 2 = highest.
pub fn terciles(values: &[(TrackId, f32)]) -> Vec<(TrackId, usize)> {
    let mut v: Vec<(TrackId, f32)> = values
        .iter()
        .copied()
        .filter(|(_, x)| x.is_finite())
        .collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let n = v.len();
    v.iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, (i * 3 / n.max(1)).min(2)))
        .collect()
}

/// Band balance of an encoded waveform (4 bytes a column: low, mid, high,
/// peak): the mean share of each band over the columns that are not
/// silent, sampled every few columns.
pub fn band_balance(waveform: &[u8]) -> Option<[f32; 3]> {
    const STEP: usize = 4;
    let mut sum = [0f64; 3];
    let mut n = 0usize;
    for c in waveform.chunks_exact(4).step_by(STEP) {
        if c[3] < 8 {
            continue;
        }
        sum[0] += c[0] as f64;
        sum[1] += c[1] as f64;
        sum[2] += c[2] as f64;
        n += 1;
    }
    let total = sum[0] + sum[1] + sum[2];
    (n > 0 && total > 0.0).then(|| {
        [
            (sum[0] / total) as f32,
            (sum[1] / total) as f32,
            (sum[2] / total) as f32,
        ]
    })
}

/// Classifies the collection. Lists without tracks are left out; tempo,
/// energy, mood and character lists keep their fixed order, genre lists
/// come largest first.
pub fn classify(tracks: &[AutoInput]) -> Vec<AutoList> {
    let mut out = Vec::new();
    let list = |group: AutoGroup, key: &str, name: &str, tracks: Vec<TrackId>| AutoList {
        group,
        key: format!("{}:{key}", group.id()),
        name: name.to_string(),
        tracks,
    };
    // Tempo.
    let mut bands: Vec<Vec<TrackId>> = vec![Vec::new(); TEMPO_BANDS.len()];
    for t in tracks {
        if let Some(i) = t.bpm.and_then(tempo_band) {
            bands[i].push(t.id);
        }
    }
    for (i, ids) in bands.into_iter().enumerate() {
        if !ids.is_empty() {
            let (_, _, key, name) = TEMPO_BANDS[i];
            out.push(list(AutoGroup::Tempo, key, name, ids));
        }
    }
    // Energy (thirds of the collection) and character (thirds by the tilt
    // of low against high band).
    let thirds = |values: Vec<(TrackId, f32)>,
                  names: [(&str, &str); 3],
                  group: AutoGroup,
                  out: &mut Vec<AutoList>| {
        let mut parts: [Vec<TrackId>; 3] = Default::default();
        for (id, third) in terciles(&values) {
            parts[third].push(id);
        }
        // Keep the collection order inside each list.
        let order: HashMap<TrackId, usize> =
            tracks.iter().enumerate().map(|(i, t)| (t.id, i)).collect();
        for (third, (key, name)) in names.iter().enumerate() {
            let mut ids = std::mem::take(&mut parts[third]);
            ids.sort_by_key(|id| order.get(id).copied().unwrap_or(usize::MAX));
            if !ids.is_empty() {
                out.push(list(group, key, name, ids));
            }
        }
    };
    thirds(
        tracks
            .iter()
            .filter_map(|t| t.profile.energy.map(|e| (t.id, e)))
            .collect(),
        [
            ("low", "Low energy"),
            ("mid", "Mid energy"),
            ("high", "High energy"),
        ],
        AutoGroup::Energy,
        &mut out,
    );
    // Mood.
    let moods = [
        ("euphoric", "Euphoric"),
        ("warm", "Warm"),
        ("driving", "Driving"),
        ("dark", "Dark"),
    ];
    let mut by_mood: HashMap<&str, Vec<TrackId>> = HashMap::new();
    for t in tracks {
        if let Some(k) = t.key {
            by_mood
                .entry(mood(k, t.profile.mood).0)
                .or_default()
                .push(t.id);
        }
    }
    for (key, name) in moods {
        if let Some(ids) = by_mood.remove(key) {
            out.push(list(AutoGroup::Mood, key, name, ids));
        }
    }
    // Character: the tilt is high minus low band, so the lowest third is bass-heavy.
    thirds(
        tracks
            .iter()
            .filter_map(|t| t.profile.bands.map(|b| (t.id, b[2] - b[0])))
            .collect(),
        [
            ("bass", "Bass-heavy"),
            ("balanced", "Balanced"),
            ("bright", "Bright"),
        ],
        AutoGroup::Character,
        &mut out,
    );
    // Genre: families, then frequent tags of their own, then the rest.
    let mut families: Vec<(String, String, Vec<TrackId>)> = Vec::new();
    let mut own: HashMap<String, (HashMap<String, usize>, Vec<TrackId>)> = HashMap::new();
    for t in tracks {
        let g = t.genre.trim();
        if g.is_empty() {
            continue;
        }
        match genre_family(g) {
            Some(f) => match families.iter_mut().find(|x| x.1 == f) {
                Some(x) => x.2.push(t.id),
                None => families.push((f.to_lowercase(), f.to_string(), vec![t.id])),
            },
            None => {
                let e = own.entry(g.to_lowercase()).or_default();
                *e.0.entry(g.to_string()).or_default() += 1;
                e.1.push(t.id);
            }
        }
    }
    let mut other = Vec::new();
    for (key, (spellings, ids)) in own {
        if ids.len() >= OWN_GENRE_MIN {
            let name = spellings
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|s| s.0)
                .unwrap_or_default();
            families.push((key, name, ids));
        } else {
            other.extend(ids);
        }
    }
    families.sort_by(|a, b| b.2.len().cmp(&a.2.len()).then(a.1.cmp(&b.1)));
    for (key, name, ids) in families {
        out.push(list(AutoGroup::Genre, &key, &name, ids));
    }
    if !other.is_empty() {
        let order: HashMap<TrackId, usize> =
            tracks.iter().enumerate().map(|(i, t)| (t.id, i)).collect();
        other.sort_by_key(|id| order.get(id).copied().unwrap_or(usize::MAX));
        out.push(list(AutoGroup::Genre, "~other", "Other genres", other));
    }
    out
}

impl Library {
    /// Classification profiles of the analysed tracks with the analysis
    /// version they were made from. Tracks whose version in `known` is
    /// current are left out (their profile has not changed), so a refresh
    /// reads only new analyses; waveforms are read one track at a time.
    pub fn auto_profiles(
        &self,
        known: &HashMap<TrackId, u32>,
    ) -> Result<HashMap<TrackId, (u32, TrackProfile)>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT track_id, version FROM analysis")?;
        let todo: Vec<(TrackId, u32)> = stmt
            .query_map([], |r| Ok((TrackId(r.get(0)?), r.get::<_, u32>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(id, v)| known.get(id) != Some(v))
            .collect();
        let mut row = self.conn.prepare_cached(
            "SELECT phrase_mood, energy, waveform FROM analysis WHERE track_id = ?1",
        )?;
        let mut out = HashMap::new();
        for (id, version) in todo {
            let (mood, energy, waveform) = row.query_row([id.0], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<Vec<u8>>>(1)?,
                    r.get::<_, Option<Vec<u8>>>(2)?,
                ))
            })?;
            let energy = energy
                .map(|e| blob::decode_f32s(&e))
                .filter(|e| !e.is_empty());
            let profile = TrackProfile {
                energy: energy.map(|e| e.iter().sum::<f32>() / e.len() as f32),
                mood: mood.and_then(|m| serde_json::from_str(&m).ok()),
                bands: waveform.as_deref().and_then(band_balance),
            };
            out.insert(id, (version, profile));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: i64, bpm: f64, key: &str, genre: &str, energy: f32, bands: [f32; 3]) -> AutoInput {
        AutoInput {
            id: TrackId(id),
            bpm: Some(bpm),
            key: MusicalKey::parse(key),
            genre: genre.into(),
            profile: TrackProfile {
                energy: Some(energy),
                mood: Some(if energy > 0.6 {
                    PhraseMood::High
                } else {
                    PhraseMood::Mid
                }),
                bands: Some(bands),
            },
        }
    }

    fn ids(lists: &[AutoList], key: &str) -> Vec<i64> {
        lists
            .iter()
            .find(|l| l.key == key)
            .map(|l| l.tracks.iter().map(|t| t.0).collect())
            .unwrap_or_default()
    }

    #[test]
    fn tempo_bands_and_genre_families() {
        assert_eq!(tempo_band(99.9), Some(0));
        assert_eq!(tempo_band(100.0), Some(1));
        assert_eq!(tempo_band(124.0), Some(3));
        assert_eq!(tempo_band(129.99), Some(3));
        assert_eq!(tempo_band(174.0), Some(6));
        assert_eq!(tempo_band(0.0), None);
        assert_eq!(genre_family("Deep House"), Some("House"));
        assert_eq!(genre_family("Afro House"), Some("House"));
        assert_eq!(genre_family("Drum & Bass"), Some("Drum & Bass"));
        assert_eq!(genre_family("DnB"), Some("Drum & Bass"));
        assert_eq!(genre_family("Melodic Techno"), Some("Techno"));
        assert_eq!(genre_family("Hip-Hop"), Some("Hip-Hop & R&B"));
        assert_eq!(genre_family("UK Garage"), Some("Garage"));
        assert_eq!(genre_family("Nu Disco"), Some("Disco & Funk"));
        assert_eq!(
            genre_family("Trapeze Music"),
            None,
            "short words stand alone"
        );
        assert_eq!(genre_family("Schlager"), None);
        assert_eq!(genre_family("  "), None);
    }

    #[test]
    fn thirds_split_by_rank() {
        let v: Vec<(TrackId, f32)> = (0..6).map(|i| (TrackId(i), i as f32 / 10.0)).collect();
        let t = terciles(&v);
        let third = |id: i64| t.iter().find(|x| x.0 == TrackId(id)).unwrap().1;
        assert_eq!([0, 1, 2, 3, 4, 5].map(third), [0, 0, 1, 1, 2, 2]);
        assert!(terciles(&[]).is_empty());
        assert_eq!(terciles(&[(TrackId(1), 0.5)]), vec![(TrackId(1), 0)]);
    }

    #[test]
    fn bands_balance_skips_silence() {
        let mut w = Vec::new();
        for _ in 0..40 {
            w.extend_from_slice(&[200, 100, 100, 220]);
            w.extend_from_slice(&[0, 0, 0, 0]);
        }
        let b = band_balance(&w).unwrap();
        assert!(
            (b[0] - 0.5).abs() < 1e-6 && (b[2] - 0.25).abs() < 1e-6,
            "{b:?}"
        );
        assert_eq!(band_balance(&[0, 0, 0, 0]), None);
    }

    #[test]
    fn classifies_the_collection() {
        let lists = classify(&[
            t(1, 122.0, "8A", "Deep House", 0.2, [0.6, 0.3, 0.1]),
            t(2, 126.0, "8B", "Tech House", 0.5, [0.4, 0.3, 0.3]),
            t(3, 128.0, "9A", "Techno", 0.9, [0.2, 0.3, 0.5]),
            t(4, 174.0, "5B", "Drum & Bass", 0.8, [0.3, 0.4, 0.3]),
            t(5, 90.0, "1A", "Schlager", 0.1, [0.5, 0.3, 0.2]),
            t(6, 95.0, "2A", "schlager", 0.3, [0.3, 0.3, 0.4]),
            t(7, 140.0, "3A", "Polka", 0.4, [0.3, 0.3, 0.4]),
        ]);
        assert_eq!(ids(&lists, "tempo:0-100"), [5, 6]);
        assert_eq!(ids(&lists, "tempo:115-124"), [1]);
        assert_eq!(ids(&lists, "tempo:124-130"), [2, 3]);
        assert_eq!(ids(&lists, "tempo:140-160"), [7]);
        assert_eq!(ids(&lists, "tempo:160-"), [4]);
        assert!(
            !lists.iter().any(|l| l.key == "tempo:100-115"),
            "empty bands are left out"
        );
        assert_eq!(ids(&lists, "energy:high"), [3, 4]);
        assert_eq!(ids(&lists, "energy:low"), [1, 5, 6]);
        assert_eq!(ids(&lists, "mood:driving"), [3]);
        assert_eq!(ids(&lists, "mood:euphoric"), [4]);
        assert_eq!(ids(&lists, "mood:warm"), [2]);
        assert_eq!(ids(&lists, "character:bass"), [1, 2, 5]);
        assert_eq!(ids(&lists, "character:bright"), [3, 7]);
        assert_eq!(ids(&lists, "genre:house"), [1, 2]);
        assert_eq!(
            ids(&lists, "genre:schlager"),
            [5, 6],
            "a frequent tag gets its own list"
        );
        assert_eq!(ids(&lists, "genre:~other"), [7]);
        let genres: Vec<&str> = lists
            .iter()
            .filter(|l| l.group == AutoGroup::Genre)
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(genres[0], "House", "largest family first: {genres:?}");
        assert_eq!(*genres.last().unwrap(), "Other genres");
        // Groups stay in tree order.
        let order: Vec<AutoGroup> = lists.iter().map(|l| l.group).collect();
        assert!(order.windows(2).all(|w| w[0] <= w[1]), "{order:?}");
    }

    #[test]
    fn profiles_come_from_the_analysis() {
        let mut lib = Library::open_in_memory().unwrap();
        let id = crate::test_util::track(&mut lib, "a", None, None);
        let a = od_core::TrackAnalysis {
            analysis_version: 3,
            energy: vec![0.2, 0.4],
            phrase_mood: PhraseMood::High,
            waveform: od_core::Waveform {
                columns_per_second: 150.0,
                columns: vec![
                    od_core::WaveformColumn {
                        low: 100,
                        mid: 50,
                        high: 50,
                        peak: 120
                    };
                    16
                ],
            },
            ..Default::default()
        };
        lib.store_analysis(id, &a, od_core::DataSource::Analysis)
            .unwrap();
        let p = lib.auto_profiles(&HashMap::new()).unwrap();
        let (v, prof) = p[&id];
        assert_eq!(v, 3);
        assert!((prof.energy.unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(prof.mood, Some(PhraseMood::High));
        assert!((prof.bands.unwrap()[0] - 0.5).abs() < 1e-6);
        // Known and current: not read again.
        assert!(
            lib.auto_profiles(&HashMap::from([(id, 3)]))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            lib.auto_profiles(&HashMap::from([(id, 2)])).unwrap().len(),
            1
        );
    }
}
