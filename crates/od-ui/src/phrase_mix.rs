//! Phrase-aligned blends ([56]): how far each deck is from its next phrase,
//! breakdown and drop, where a phrase-snapped sync puts the incoming deck,
//! the beat a cued start waits for, and where the incoming track is drawn
//! over the playing one.
//!
//! All functions are pure and work in grid beats (the beat index of the
//! track's beatgrid, fractional, as [`od_engine::DeckState::beat`]).

use od_core::{Phrase, PhraseKind};

/// Beats per bar assumed for phrase counts.
const BAR: f64 = 4.0;
/// Without analysed phrases the grid's 16-bar marks count as phrases (the
/// amber ticks on the waveform).
pub(crate) const FALLBACK_PHRASE: f64 = 64.0;

/// The beats where a phrase starts (and the end of the last one), sorted.
/// Without phrases: `None`.
fn boundaries(phrases: &[Phrase]) -> Option<Vec<f64>> {
    if phrases.is_empty() {
        return None;
    }
    let mut b: Vec<f64> = phrases.iter().map(|p| p.start_beat as f64).collect();
    if let Some(last) = phrases.iter().map(|p| p.end_beat).max() {
        b.push(last as f64);
    }
    b.sort_by(f64::total_cmp);
    b.dedup();
    Some(b)
}

/// The first phrase boundary after `beat` (strictly, by more than `margin`
/// beats); the next 16-bar mark when the track has no phrases or is past
/// its last one.
pub(crate) fn next_boundary(phrases: &[Phrase], beat: f64, margin: f64) -> f64 {
    let after = beat + margin;
    boundaries(phrases)
        .and_then(|b| b.into_iter().find(|&x| x > after))
        .unwrap_or_else(|| ((after / FALLBACK_PHRASE).floor() + 1.0) * FALLBACK_PHRASE)
}

/// What counts as a breakdown: the quiet middle sections.
fn is_breakdown(k: PhraseKind) -> bool {
    matches!(k, PhraseKind::Down | PhraseKind::Bridge)
}

/// What counts as a drop: a chorus that starts after something else.
fn is_drop(prev: Option<PhraseKind>, k: PhraseKind) -> bool {
    k == PhraseKind::Chorus && prev != Some(PhraseKind::Chorus)
}

/// Whole bars (rounded up) from `beat` to `target`.
fn bars_until(beat: f64, target: f64) -> u32 {
    ((target - beat) / BAR - 1e-6).ceil().max(1.0) as u32
}

/// Bars to the next drop and the next breakdown after `beat` (`None` when
/// the track has none ahead).
pub(crate) fn ahead(phrases: &[Phrase], beat: f64) -> (Option<u32>, Option<u32>) {
    let mut sorted = phrases.to_vec();
    sorted.sort_by_key(|p| p.start_beat);
    let mut drop = None;
    let mut breakdown = None;
    let mut prev: Option<PhraseKind> = None;
    for p in &sorted {
        let start = p.start_beat as f64;
        if start > beat + 1e-6 {
            if drop.is_none() && is_drop(prev, p.kind) {
                drop = Some(bars_until(beat, start));
            }
            if breakdown.is_none() && is_breakdown(p.kind) {
                breakdown = Some(bars_until(beat, start));
            }
        }
        prev = Some(p.kind);
    }
    (drop, breakdown)
}

/// The deck's look-ahead line, nearest first: "DROP 8 · BREAK 24".
pub(crate) fn ahead_text(phrases: &[Phrase], beat: f64) -> String {
    let (drop, brk) = ahead(phrases, beat);
    let mut parts: Vec<(u32, String)> = Vec::new();
    if let Some(b) = drop {
        parts.push((b, od_i18n::trf("DROP {}", &[&b]).to_string()));
    }
    if let Some(b) = brk {
        parts.push((b, od_i18n::trf("BREAK {}", &[&b]).to_string()));
    }
    parts.sort_by_key(|p| p.0);
    parts
        .into_iter()
        .map(|p| p.1)
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Phrase-snapped sync: the whole number of beats the incoming deck jumps
/// so that it stands as far before one of its phrase boundaries as the
/// master stands before its next one. The smallest such jump; `None` when
/// it already does (within half a beat).
pub(crate) fn sync_jump(
    master_phrases: &[Phrase],
    master_beat: f64,
    phrases: &[Phrase],
    beat: f64,
) -> Option<f64> {
    let left = next_boundary(master_phrases, master_beat, 0.0) - master_beat;
    let candidates: Vec<f64> = match boundaries(phrases) {
        Some(b) => b,
        None => {
            let k = (beat / FALLBACK_PHRASE).floor();
            (-1..=2).map(|i| (k + i as f64) * FALLBACK_PHRASE).collect()
        }
    };
    let best = candidates
        .into_iter()
        .map(|b| (b - left - beat).round())
        // Never before the start of the track.
        .filter(|j| beat + j >= 0.0)
        .min_by(|a, b| a.abs().total_cmp(&b.abs()).then(b.total_cmp(a)))?;
    (best.abs() >= 1.0).then_some(best)
}

/// Bars and beats to a start that waits for `target` on the master clock,
/// as the bar counter shows them ("-3.2").
pub(crate) fn wait_text(master_beat: f64, target: f64) -> String {
    crate::format::bar_countdown(master_beat, target)
}

/// Where the incoming deck's track is drawn over the playing one: its
/// position (track seconds) at the master's playhead now and the rate it
/// moves at (track seconds per real second). A playing deck is drawn where
/// it is; a waiting (cued) one so that its cue meets the master at
/// `start_beat`, the beat it would start on.
pub(crate) fn incoming_position(
    position: f64,
    playing: bool,
    rate: f64,
    master_beat: f64,
    master_bpm: f64,
    start_beat: f64,
) -> f64 {
    if playing || master_bpm <= 0.0 {
        return position;
    }
    let wait = (start_beat - master_beat).max(0.0) * 60.0 / master_bpm;
    position - wait * rate
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(start: u32, end: u32, kind: PhraseKind) -> Phrase {
        Phrase::new(start, end, kind)
    }

    /// Intro 16 bars, up 8, chorus 16, down 8, chorus 16, outro 8.
    fn track() -> Vec<Phrase> {
        vec![
            p(0, 64, PhraseKind::Intro),
            p(64, 96, PhraseKind::Up),
            p(96, 160, PhraseKind::Chorus),
            p(160, 192, PhraseKind::Down),
            p(192, 256, PhraseKind::Chorus),
            p(256, 288, PhraseKind::Outro),
        ]
    }

    #[test]
    fn next_boundary_follows_phrases_or_16_bars() {
        let t = track();
        assert_eq!(next_boundary(&t, 10.0, 0.0), 64.0);
        assert_eq!(next_boundary(&t, 64.0, 0.0), 96.0);
        assert_eq!(next_boundary(&t, 63.9, 0.25), 96.0, "too close: the next one");
        assert_eq!(next_boundary(&t, 270.0, 0.0), 288.0, "end of the last phrase");
        assert_eq!(next_boundary(&t, 300.0, 0.0), 320.0, "past the phrases: 16 bars");
        assert_eq!(next_boundary(&[], 70.0, 0.0), 128.0);
    }

    #[test]
    fn bars_to_drop_and_breakdown() {
        let t = track();
        assert_eq!(ahead(&t, 0.0), (Some(24), Some(40)));
        // One beat into the intro: still 24 bars (rounded up).
        assert_eq!(ahead(&t, 1.0), (Some(24), Some(40)));
        // In the first chorus: the breakdown comes first, then the second drop.
        assert_eq!(ahead(&t, 100.0), (Some(23), Some(15)));
        assert_eq!(ahead(&t, 200.0), (None, None));
        assert_eq!(ahead_text(&t, 100.0), "BREAK 15 · DROP 23");
        assert_eq!(ahead_text(&t, 200.0), "");
        assert_eq!(ahead_text(&[], 0.0), "");
        // A chorus right after a chorus is no new drop.
        let joined = vec![
            p(0, 32, PhraseKind::Chorus),
            p(32, 64, PhraseKind::Chorus),
            p(64, 96, PhraseKind::Bridge),
        ];
        assert_eq!(ahead(&joined, 0.0), (None, Some(16)));
    }

    #[test]
    fn phrase_sync_lines_up_the_phrase_positions() {
        let t = track();
        // Master 8 beats before its next phrase; the incoming deck is 4
        // beats into its intro: it jumps to 8 before its first boundary.
        assert_eq!(sync_jump(&t, 56.0, &t, 4.0), Some(52.0));
        // 8 beats before a boundary already: nothing to do.
        assert_eq!(sync_jump(&t, 56.0, &t, 88.0), None);
        // The nearest boundary wins, backwards too.
        assert_eq!(sync_jump(&t, 56.0, &t, 158.0), Some(-6.0));
        // Fractional beats round to whole ones (sync keeps the phase).
        assert_eq!(sync_jump(&t, 56.3, &t, 4.0), Some(52.0));
        // Without phrases the 16-bar grid stands in.
        assert_eq!(sync_jump(&[], 60.0, &[], 10.0), Some(50.0));
    }

    #[test]
    fn a_waiting_deck_is_drawn_at_its_start() {
        // Master at 128 BPM, 8 beats (3.75 s) before the phrase; the cued
        // deck runs at the master's rate 1.0: its cue sits 3.75 s ahead.
        let pos = incoming_position(30.0, false, 1.0, 56.0, 128.0, 64.0);
        assert!((pos - 26.25).abs() < 1e-9);
        // Playing: where it is.
        assert_eq!(incoming_position(30.0, true, 1.0, 56.0, 128.0, 64.0), 30.0);
        assert_eq!(wait_text(56.0, 64.0), "-1.4");
    }
}
