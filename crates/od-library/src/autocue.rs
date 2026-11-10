//! Personalised cue points: learns where the user puts hot cues and memory
//! cues in their analysed tracks — relative to the first downbeat, the
//! phrases (first / second Intro, Chorus, Up, …) and the last downbeat, in
//! whole bars — and places the same pattern, with its names, colours and
//! loop lengths, on tracks that have none.
//!
//! [`learn`] and [`apply`] are pure; [`Library::learn_cue_pattern`] reads
//! the examples and [`Library::set_cues_like`] applies the pattern to one
//! track as one undo step (the change log then updates the sticks).

use crate::journal::EntityState;
use crate::tracks::{read_cues, read_grid, read_phrases};
use crate::{Library, Result};
use od_core::cue::HOT_CUE_SLOTS;
use od_core::{Beatgrid, Cue, CueColor, CueKind, DataSource, Phrase, PhraseKind, TrackId};
use std::collections::HashMap;

/// A place in a track a cue is measured from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// The first downbeat of the grid.
    FirstDownbeat,
    /// The start of the n-th (0-based) section of this kind.
    Phrase(PhraseKind, u8),
    /// The last downbeat of the grid.
    LastDownbeat,
}

const KINDS: [PhraseKind; 7] = [
    PhraseKind::Intro,
    PhraseKind::Verse,
    PhraseKind::Chorus,
    PhraseKind::Bridge,
    PhraseKind::Up,
    PhraseKind::Down,
    PhraseKind::Outro,
];

impl Anchor {
    /// Preference among equally frequent explanations of a cue: the grid
    /// first, then early phrases, the end last.
    fn rank(self) -> u32 {
        match self {
            Anchor::FirstDownbeat => 0,
            Anchor::Phrase(k, n) => {
                1 + n as u32 * 8 + KINDS.iter().position(|x| *x == k).unwrap_or(7) as u32
            }
            Anchor::LastDownbeat => 100,
        }
    }

    /// English description ("first downbeat", "2nd Chorus").
    pub fn describe(self) -> String {
        match self {
            Anchor::FirstDownbeat => "first downbeat".into(),
            Anchor::LastDownbeat => "last downbeat".into(),
            Anchor::Phrase(k, n) => {
                let kind = format!("{k:?}");
                if n == 0 {
                    format!("first {kind}")
                } else {
                    format!("{} {kind}", ["1st", "2nd", "3rd"][(n as usize).min(2)])
                }
            }
        }
    }
}

/// One learned cue: what kind, where (anchor plus whole bars), and how it
/// looks.
#[derive(Clone, Debug, PartialEq)]
pub struct CueRule {
    pub kind: CueKind,
    pub anchor: Anchor,
    /// Bars from the anchor (negative: before it).
    pub bars: i32,
    pub label: String,
    pub color: Option<CueColor>,
    /// A stored loop of this many beats.
    pub loop_beats: Option<u32>,
    /// Example tracks that follow the rule.
    pub support: usize,
}

impl CueRule {
    /// English description, e.g. "A at the first downbeat",
    /// "memory cue 16 bars before the first Outro".
    pub fn describe(&self) -> String {
        let what = match self.kind {
            CueKind::Hot(_) => self.kind.hot_slot_letter().unwrap_or('?').to_string(),
            CueKind::Memory => "memory cue".to_string(),
        };
        let at = self.anchor.describe();
        match self.bars {
            0 => format!("{what} at the {at}"),
            b if b < 0 => format!("{what} {} bars before the {at}", -b),
            b => format!("{what} {b} bars after the {at}"),
        }
    }
}

/// The learned pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct CuePattern {
    pub rules: Vec<CueRule>,
    /// Example tracks it was learned from.
    pub examples: usize,
}

impl CuePattern {
    pub fn describe(&self) -> String {
        self.rules
            .iter()
            .map(CueRule::describe)
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// An analysed track with the user's cues.
#[derive(Clone, Debug, Default)]
pub struct CueExample {
    pub grid: Beatgrid,
    pub phrases: Vec<Phrase>,
    pub cues: Vec<Cue>,
}

/// Fewest example tracks a pattern is learned from.
pub const MIN_EXAMPLES: usize = 3;
/// Most example tracks read (the most recently added ones).
const MAX_EXAMPLES: usize = 400;
/// How far off a bar line a cue may sit and still count as on it (beats).
const TOLERANCE: f64 = 0.5;
/// Farthest a cue is measured from an anchor (bars).
const MAX_BARS: i32 = 128;

/// Anchors of a track and their beat index in its grid.
pub fn anchors(grid: &Beatgrid, phrases: &[Phrase]) -> Vec<(Anchor, f64)> {
    let mut out = Vec::new();
    let downbeats = || {
        grid.beats
            .iter()
            .enumerate()
            .filter(|(_, b)| b.bar_position == 1)
    };
    if let Some((i, _)) = downbeats().next() {
        out.push((Anchor::FirstDownbeat, i as f64));
    }
    let mut seen: HashMap<PhraseKind, u8> = HashMap::new();
    for p in od_core::normalize_phrases(phrases) {
        let n = seen.entry(p.kind).or_default();
        if *n < 3 {
            out.push((Anchor::Phrase(p.kind, *n), p.start_beat as f64));
        }
        *n += 1;
    }
    if let Some((i, _)) = downbeats().last() {
        out.push((Anchor::LastDownbeat, i as f64));
    }
    out
}

/// Every (anchor, bars) that places a cue at beat `beat`.
fn explain(beat: f64, anchors: &[(Anchor, f64)]) -> Vec<(Anchor, i32)> {
    anchors
        .iter()
        .filter_map(|(a, at)| {
            let d = beat - at;
            let bars = (d / 4.0).round();
            ((d - bars * 4.0).abs() <= TOLERANCE && bars.abs() <= MAX_BARS as f64)
                .then_some((*a, bars as i32))
        })
        .collect()
}

/// The most common value (first seen wins a tie).
fn mode<T: Clone + PartialEq>(values: impl IntoIterator<Item = T>) -> Option<T> {
    let mut counts: Vec<(T, usize)> = Vec::new();
    for v in values {
        match counts.iter_mut().find(|(x, _)| *x == v) {
            Some(c) => c.1 += 1,
            None => counts.push((v, 1)),
        }
    }
    let best = counts.iter().map(|c| c.1).max()?;
    counts.into_iter().find(|c| c.1 == best).map(|c| c.0)
}

/// At least half of `n`, and at least two.
fn majority(n: usize) -> usize {
    n.div_ceil(2).max(2)
}

/// One cue of an example, measured.
struct Measured<'a> {
    cue: &'a Cue,
    explain: Vec<(Anchor, i32)>,
    loop_beats: Option<u32>,
}

fn measure<'a>(
    ex: &'a CueExample,
    cue: &'a Cue,
    anchors: &[(Anchor, f64)],
) -> Option<Measured<'a>> {
    let beat = ex.grid.beat_position(cue.start)?;
    let loop_beats = cue
        .loop_end
        .and_then(|e| ex.grid.beat_position(e))
        .map(|e| (e - beat).round().max(1.0) as u32);
    Some(Measured {
        cue,
        explain: explain(beat, anchors),
        loop_beats,
    })
}

/// The best explanation shared by the most tracks: (anchor, bars, tracks).
fn best(per_track: &[Vec<Vec<(Anchor, i32)>>]) -> Option<((Anchor, i32), usize)> {
    let mut counts: HashMap<(Anchor, i32), usize> = HashMap::new();
    for cues in per_track {
        let mut once: Vec<(Anchor, i32)> = cues.iter().flatten().copied().collect();
        once.sort_by_key(|(a, b)| (a.rank(), *b));
        once.dedup();
        for e in once {
            *counts.entry(e).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .min_by_key(|((a, b), n)| (std::cmp::Reverse(*n), a.rank(), b.abs(), *b))
}

fn rule(
    kind: CueKind,
    (anchor, bars): (Anchor, i32),
    support: usize,
    matching: &[&Measured],
) -> CueRule {
    CueRule {
        kind,
        anchor,
        bars,
        label: mode(matching.iter().map(|m| m.cue.label.clone())).unwrap_or_default(),
        color: mode(matching.iter().map(|m| m.cue.color)).flatten(),
        loop_beats: mode(matching.iter().map(|m| m.loop_beats)).flatten(),
        support,
    }
}

/// Learns the pattern of the examples' cues (cues placed by an earlier
/// application of a pattern are not examples). `None` with fewer than
/// [`MIN_EXAMPLES`] analysed tracks with cues, or when no cue sits at the
/// same place in most of them.
pub fn learn(examples: &[CueExample]) -> Option<CuePattern> {
    let examples: Vec<(&CueExample, Vec<(Anchor, f64)>)> = examples
        .iter()
        .filter(|e| !e.grid.is_empty() && e.cues.iter().any(|c| c.source != DataSource::Analysis))
        .map(|e| (e, anchors(&e.grid, &e.phrases)))
        .collect();
    let n = examples.len();
    if n < MIN_EXAMPLES {
        return None;
    }
    let measured: Vec<Vec<Measured>> = examples
        .iter()
        .map(|(e, a)| {
            e.cues
                .iter()
                .filter(|c| c.source != DataSource::Analysis)
                .filter_map(|c| measure(e, c, a))
                .collect()
        })
        .collect();
    let mut rules = Vec::new();
    // Hot cues: per pad, the place most tracks agree on.
    for slot in 0..HOT_CUE_SLOTS {
        let on_pad: Vec<&Measured> = measured
            .iter()
            .filter_map(|cues| cues.iter().find(|m| m.cue.kind == CueKind::Hot(slot)))
            .collect();
        if on_pad.len() < majority(n) {
            continue;
        }
        let per_track: Vec<Vec<Vec<(Anchor, i32)>>> =
            on_pad.iter().map(|m| vec![m.explain.clone()]).collect();
        let Some((place, support)) = best(&per_track) else {
            continue;
        };
        if support < majority(on_pad.len()) {
            continue;
        }
        let matching: Vec<&Measured> = on_pad
            .iter()
            .copied()
            .filter(|m| m.explain.contains(&place))
            .collect();
        rules.push(rule(CueKind::Hot(slot), place, support, &matching));
    }
    // Memory cues: places shared by most tracks with memory cues, the most
    // common first; each cue explains one place.
    let mut memory: Vec<Vec<&Measured>> = measured
        .iter()
        .map(|cues| {
            cues.iter()
                .filter(|m| m.cue.kind == CueKind::Memory)
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .collect();
    let with_memory = memory.len();
    if with_memory >= majority(n) {
        for _ in 0..8 {
            let per_track: Vec<Vec<Vec<(Anchor, i32)>>> = memory
                .iter()
                .map(|cues| cues.iter().map(|m| m.explain.clone()).collect())
                .collect();
            let Some((place, support)) = best(&per_track) else {
                break;
            };
            if support < majority(with_memory) {
                break;
            }
            let mut matching = Vec::new();
            for cues in memory.iter_mut() {
                if let Some(i) = cues.iter().position(|m| m.explain.contains(&place)) {
                    matching.push(cues.remove(i));
                }
            }
            rules.push(rule(CueKind::Memory, place, support, &matching));
        }
    }
    (!rules.is_empty()).then_some(CuePattern { rules, examples: n })
}

/// The cues `pattern` adds to a track with this grid, phrases and cues: hot
/// cues on free pads only, memory cues where there is none yet. Rules
/// whose anchor the track lacks (no Chorus, …) or that fall outside the
/// grid are skipped.
pub fn apply(
    pattern: &CuePattern,
    grid: &Beatgrid,
    phrases: &[Phrase],
    existing: &[Cue],
) -> Vec<Cue> {
    let anchors = anchors(grid, phrases);
    let last = grid.beats.len().saturating_sub(1) as f64;
    let mut out: Vec<Cue> = Vec::new();
    for r in &pattern.rules {
        let Some((_, at)) = anchors.iter().find(|(a, _)| *a == r.anchor) else {
            continue;
        };
        let beat = at + r.bars as f64 * 4.0;
        if beat < 0.0 || beat > last {
            continue;
        }
        let Some(start) = grid.time_of_beat(beat) else {
            continue;
        };
        let taken = existing.iter().chain(out.iter()).any(|c| match r.kind {
            CueKind::Hot(_) => c.kind == r.kind,
            CueKind::Memory => c.kind == CueKind::Memory && (c.start - start).abs() < 0.05,
        });
        if taken {
            continue;
        }
        out.push(Cue {
            kind: r.kind,
            start,
            loop_end: r
                .loop_beats
                .and_then(|b| grid.time_of_beat((beat + b as f64).min(last))),
            label: r.label.clone(),
            color: r.color,
            source: DataSource::Analysis,
            active: false,
        });
    }
    out
}

impl Library {
    /// Analysed tracks with cues, the most recently added first (at most a
    /// few hundred).
    pub fn cue_examples(&self) -> Result<Vec<CueExample>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT DISTINCT c.track_id FROM cue c JOIN analysis a ON a.track_id = c.track_id
             WHERE a.grid IS NOT NULL ORDER BY c.track_id DESC LIMIT ?1",
        )?;
        let ids: Vec<TrackId> = stmt
            .query_map([MAX_EXAMPLES as i64], |r| Ok(TrackId(r.get(0)?)))?
            .collect::<rusqlite::Result<_>>()?;
        ids.into_iter()
            .map(|id| {
                Ok(CueExample {
                    grid: read_grid(&self.conn, id)?.map(|g| g.0).unwrap_or_default(),
                    phrases: read_phrases(&self.conn, id)?,
                    cues: read_cues(&self.conn, id)?,
                })
            })
            .collect()
    }

    /// The user's cue pattern, if there are enough examples.
    pub fn learn_cue_pattern(&self) -> Result<Option<CuePattern>> {
        Ok(learn(&self.cue_examples()?))
    }

    /// Places `pattern` on track `id` (see [`apply`]): one undo step, `0`
    /// and no edit when nothing fits (no beatgrid, every pad taken, …).
    pub fn set_cues_like(&mut self, id: TrackId, pattern: &CuePattern) -> Result<usize> {
        let Some((grid, _)) = read_grid(&self.conn, id)? else {
            return Ok(0);
        };
        let phrases = read_phrases(&self.conn, id)?;
        let mut cues = read_cues(&self.conn, id)?;
        let added = apply(pattern, &grid, &phrases, &cues);
        if added.is_empty() {
            return Ok(0);
        }
        let n = added.len();
        cues.extend(added);
        self.edit(
            "Set cues like my others",
            vec![EntityState::Cues { track: id, cues }],
        )?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::PhraseKind::*;

    /// A 128 BPM grid whose first downbeat is the third beat, 400 beats.
    fn grid() -> Beatgrid {
        Beatgrid::constant(128.0, 1.0, 1, 400.0 * 60.0 / 128.0 + 0.5)
    }

    fn time(g: &Beatgrid, beat: f64) -> f64 {
        g.time_of_beat(beat).unwrap()
    }

    /// Phrases in bars from the first downbeat (beat 2).
    fn phrases(sections: &[(u32, u32, PhraseKind)]) -> Vec<Phrase> {
        sections
            .iter()
            .map(|(s, e, k)| Phrase::new(2 + s * 4, 2 + e * 4, *k))
            .collect()
    }

    fn hot(g: &Beatgrid, slot: u8, beat: f64, label: &str, color: Option<CueColor>) -> Cue {
        Cue {
            label: label.into(),
            color,
            ..Cue::hot(slot, time(g, beat))
        }
    }

    /// Tracks whose chorus starts at a different bar: A on the first
    /// downbeat, B on the first chorus (red, "DROP"), a memory cue 16 bars
    /// before the outro (which moves too); one track with a stray extra cue.
    fn examples() -> Vec<CueExample> {
        let red = Some(CueColor::rgb(230, 40, 40));
        [(32u32, 80u32), (16, 72), (24, 88), (32, 76)]
            .iter()
            .enumerate()
            .map(|(i, (chorus, outro))| {
                let g = grid();
                let p = phrases(&[
                    (0, *chorus, Intro),
                    (*chorus, *outro, Chorus),
                    (*outro, 96, Outro),
                ]);
                let mut cues = vec![
                    hot(&g, 0, 2.0, "", None),
                    hot(&g, 1, 2.0 + *chorus as f64 * 4.0, "DROP", red),
                    Cue::memory(time(&g, 2.0 + (*outro as f64 - 16.0) * 4.0)),
                ];
                if i == 3 {
                    cues.push(Cue::memory(time(&g, 2.0 + 5.0 * 4.0 + 1.0)));
                }
                CueExample {
                    grid: g,
                    phrases: p,
                    cues,
                }
            })
            .collect()
    }

    #[test]
    fn learns_cues_relative_to_phrases_and_bars() {
        let p = learn(&examples()).expect("a pattern");
        assert_eq!(p.examples, 4);
        let by_kind = |k: CueKind| p.rules.iter().find(|r| r.kind == k).unwrap();
        let a = by_kind(CueKind::Hot(0));
        assert_eq!((a.anchor, a.bars), (Anchor::FirstDownbeat, 0));
        let b = by_kind(CueKind::Hot(1));
        assert_eq!((b.anchor, b.bars), (Anchor::Phrase(Chorus, 0), 0));
        assert_eq!(b.label, "DROP");
        assert_eq!(b.color, Some(CueColor::rgb(230, 40, 40)));
        let m: Vec<&CueRule> = p
            .rules
            .iter()
            .filter(|r| r.kind == CueKind::Memory)
            .collect();
        assert_eq!(m.len(), 1, "the stray cue is no rule: {}", p.describe());
        assert_eq!((m[0].anchor, m[0].bars), (Anchor::Phrase(Outro, 0), -16));
        assert_eq!(
            p.describe(),
            "A at the first downbeat · B at the first Chorus · memory cue 16 bars before the first Outro"
        );
        // Too few examples, or no agreement: no pattern.
        assert!(learn(&examples()[..2]).is_none());
        let g = grid();
        let scattered: Vec<CueExample> = (0..4)
            .map(|i| CueExample {
                grid: g.clone(),
                phrases: vec![],
                cues: vec![Cue::hot(i as u8, time(&g, 2.0 + i as f64 * 37.0 + 1.0))],
            })
            .collect();
        assert!(learn(&scattered).is_none());
    }

    #[test]
    fn applies_the_pattern_to_a_new_track() {
        let p = learn(&examples()).unwrap();
        let g = grid();
        // Chorus at bar 8, outro at bar 72.
        let ph = phrases(&[(0, 8, Intro), (8, 72, Chorus), (72, 90, Outro)]);
        let cues = apply(&p, &g, &ph, &[]);
        let at = |k: CueKind| cues.iter().find(|c| c.kind == k).map(|c| c.start);
        assert_eq!(at(CueKind::Hot(0)), Some(time(&g, 2.0)));
        assert_eq!(at(CueKind::Hot(1)), Some(time(&g, 2.0 + 32.0)));
        assert_eq!(at(CueKind::Memory), Some(time(&g, 2.0 + 56.0 * 4.0)));
        let b = cues.iter().find(|c| c.kind == CueKind::Hot(1)).unwrap();
        assert_eq!(b.label, "DROP");
        assert!(cues.iter().all(|c| c.source == DataSource::Analysis));
        // Taken pads and existing memory cues stay; a missing anchor is skipped.
        let existing = vec![Cue::hot(0, 50.0), Cue::memory(time(&g, 2.0 + 56.0 * 4.0))];
        let more = apply(&p, &g, &ph, &existing);
        assert_eq!(
            more.iter().map(|c| c.kind).collect::<Vec<_>>(),
            [CueKind::Hot(1)]
        );
        let no_chorus = apply(&p, &g, &phrases(&[(0, 90, Intro)]), &[]);
        assert!(no_chorus.iter().all(|c| c.kind != CueKind::Hot(1)));
        // Cues placed by a pattern are not examples.
        let mut ex = examples();
        for e in &mut ex {
            for c in &mut e.cues {
                c.source = DataSource::Analysis;
            }
        }
        assert!(learn(&ex).is_none());
    }

    #[test]
    fn learned_loops_keep_their_length() {
        let mut ex = examples();
        for e in &mut ex {
            let end = time(&e.grid, 2.0 + 8.0);
            e.cues[0].loop_end = Some(end);
        }
        let p = learn(&ex).unwrap();
        assert_eq!(p.rules[0].loop_beats, Some(8));
        let g = grid();
        let c = apply(&p, &g, &[], &[]);
        assert_eq!(c[0].loop_end, Some(time(&g, 10.0)));
    }

    #[test]
    fn library_learns_and_applies_as_one_undo_step() {
        let mut lib = Library::open_in_memory().unwrap();
        let mk = |lib: &mut Library, name: &str, ph: &[Phrase]| {
            let id = crate::test_util::track(lib, name, Some(128.0), None);
            lib.store_analysis(
                id,
                &od_core::TrackAnalysis {
                    beatgrid: grid(),
                    phrases: ph.to_vec(),
                    ..Default::default()
                },
                DataSource::Analysis,
            )
            .unwrap();
            id
        };
        for (i, e) in examples().into_iter().enumerate() {
            let id = mk(&mut lib, &format!("ex{i}"), &e.phrases);
            lib.set_cues(id, e.cues).unwrap();
        }
        let fresh = mk(
            &mut lib,
            "fresh",
            &phrases(&[(0, 8, Intro), (8, 72, Chorus), (72, 90, Outro)]),
        );
        let p = lib.learn_cue_pattern().unwrap().expect("pattern");
        let seq = lib.change_seq().unwrap();
        assert_eq!(lib.set_cues_like(fresh, &p).unwrap(), 3);
        assert_eq!(lib.cues(fresh).unwrap().len(), 3);
        assert!(lib.change_seq().unwrap() > seq, "change log: sticks resync");
        assert_eq!(
            lib.set_cues_like(fresh, &p).unwrap(),
            0,
            "nothing left to add"
        );
        assert_eq!(
            lib.undo().unwrap().as_deref(),
            Some("Set cues like my others")
        );
        assert!(lib.cues(fresh).unwrap().is_empty());
    }
}
