//! Phase and tempo between decks (roadmap [12]): each deck with a beatgrid
//! is measured against a reference deck (the sync master, else the first
//! deck that plays, else the first loaded one). The phase is where the
//! deck's beat falls against the reference beat, within half a beat each
//! way; the tempo difference is the difference of the effective BPMs.
//!
//! Practice turns SYNC off on every deck and keeps it off, so beatmatching
//! is done by ear and from this meter.

use crate::app::{App, handler};
use od_engine::{DeckCommand, DeckState, EngineState};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::Cell;
use std::rc::Rc;

/// In sync: tempo within this many BPM ...
pub(crate) const SYNC_BPM: f64 = 0.05;
/// ... and beats within this fraction of a beat (about 10 ms at 125 BPM).
pub(crate) const SYNC_BEAT: f64 = 0.02;

/// One deck against the reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Compare {
    /// Beats, -0.5..=0.5; positive when the deck's beat comes first.
    pub offset: f64,
    /// Milliseconds at the reference tempo.
    pub ms: f64,
    /// Effective BPM of the deck minus the reference's.
    pub tempo: f64,
    pub in_sync: bool,
}

/// Fraction of the beat at the playhead.
fn phase(d: &DeckState) -> f64 {
    d.beat.rem_euclid(1.0)
}

fn has_grid(d: &DeckState) -> bool {
    d.track.is_some() && d.bpm > 0.0 && d.track_bpm > 0.0
}

/// `deck` against `reference`; `None` without grids on both.
pub(crate) fn compare(reference: &DeckState, deck: &DeckState) -> Option<Compare> {
    if !has_grid(reference) || !has_grid(deck) {
        return None;
    }
    let mut offset = phase(deck) - phase(reference);
    if offset > 0.5 {
        offset -= 1.0;
    } else if offset <= -0.5 {
        offset += 1.0;
    }
    let tempo = deck.bpm - reference.bpm;
    let ms = offset * 60_000.0 / reference.bpm;
    Some(Compare {
        offset,
        ms,
        tempo,
        in_sync: tempo.abs() < SYNC_BPM && offset.abs() < SYNC_BEAT,
    })
}

/// The deck the others are measured against (0-based), among the decks
/// `shown` in Perform.
pub(crate) fn reference(state: &EngineState, shown: &[usize]) -> Option<usize> {
    let decks: Vec<usize> = shown.iter().copied().filter(|&d| d < state.decks.len()).collect();
    state
        .master_deck
        .filter(|m| decks.contains(m) && has_grid(&state.decks[*m]))
        .or_else(|| decks.iter().copied().find(|&i| state.decks[i].playing && has_grid(&state.decks[i])))
        .or_else(|| decks.iter().copied().find(|&i| has_grid(&state.decks[i])))
}

/// "+12 ms", "−4 ms".
fn ms_text(ms: f64) -> String {
    let r = ms.round() as i64;
    match r {
        0 => "0 ms".into(),
        r if r > 0 => format!("+{r} ms"),
        r => format!("−{} ms", -r),
    }
}

/// "+0.40 BPM", "±0.00 BPM".
fn tempo_text(t: f64) -> String {
    if t.abs() < 0.005 {
        "±0.00 BPM".into()
    } else if t > 0.0 {
        format!("+{t:.2} BPM")
    } else {
        format!("−{:.2} BPM", -t)
    }
}

thread_local! {
    static PRACTICE: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn practice() -> bool {
    PRACTICE.with(|p| p.get())
}

pub(crate) fn set_practice(app: &App, on: bool) {
    PRACTICE.with(|p| p.set(on));
    app.ui.global::<crate::PhaseMeter>().set_practice(on);
    if on {
        for d in 0..crate::decks::DECKS {
            crate::engine::deck_cmd(app, d, DeckCommand::SetSync(false));
        }
        app.status(od_i18n::tr("Practice: SYNC is off. Match tempo and phase by ear and with the phase meter"));
    } else {
        app.status(od_i18n::tr("SYNC can be used again"));
    }
}

/// Frame tick (Perform): the rows of the meter.
pub(crate) fn project(app: &App, state: &EngineState) {
    // The decks the lanes above show (in the 2-deck view, those a
    // four-deck controller has active).
    let shown = app.decks.borrow().shown();
    let g = app.ui.global::<crate::PhaseMeter>();
    let Some(r) = reference(state, &shown) else {
        g.set_reference(0);
        if g.get_rows().row_count() > 0 {
            g.set_rows(ModelRc::default());
        }
        return;
    };
    let rows: Vec<crate::PhaseRow> = shown
        .iter()
        .copied()
        .filter(|&d| d < state.decks.len() && d != r)
        .filter_map(|d| {
            let c = compare(&state.decks[r], &state.decks[d])?;
            Some(crate::PhaseRow {
                deck: d as i32 + 1,
                offset: c.offset as f32,
                ms: ms_text(c.ms).into(),
                tempo: tempo_text(c.tempo).into(),
                in_sync: c.in_sync,
            })
        })
        .collect();
    g.set_reference(r as i32 + 1);
    // Same row count: update in place (no relayout every frame).
    let model = g.get_rows();
    if model.row_count() == rows.len() {
        for (i, row) in rows.into_iter().enumerate() {
            if model.row_data(i).as_ref() != Some(&row) {
                model.set_row_data(i, row);
            }
        }
    } else {
        g.set_rows(ModelRc::new(VecModel::from(rows)));
    }
}

use slint::Model;

pub(crate) fn wire(app: &Rc<App>) {
    app.ui
        .global::<crate::PhaseMeter>()
        .on_toggle_practice(handler!(app, |a| set_practice(&a, !practice())));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(beat: f64, bpm: f64) -> DeckState {
        DeckState {
            track: Some(od_core::TrackId(1)),
            beat,
            bpm,
            track_bpm: 124.0,
            playing: true,
            ..DeckState::default()
        }
    }

    #[test]
    fn phase_wraps_to_half_a_beat_each_way() {
        let r = deck(10.10, 124.0);
        let c = compare(&r, &deck(3.30, 124.0)).unwrap();
        assert!((c.offset - 0.2).abs() < 1e-9, "ahead by 0.2 beat: {}", c.offset);
        assert!((c.ms - 0.2 * 60_000.0 / 124.0).abs() < 1e-6);
        let c = compare(&r, &deck(7.95, 124.0)).unwrap();
        assert!((c.offset + 0.15).abs() < 1e-9, "behind by 0.15 beat: {}", c.offset);
        let c = compare(&r, &deck(7.70, 124.0)).unwrap();
        assert!((c.offset + 0.4).abs() < 1e-9, "{}", c.offset);
        let c = compare(&r, &deck(7.55, 124.0)).unwrap();
        assert!((c.offset - 0.45).abs() < 1e-9, "0.55 behind is 0.45 ahead: {}", c.offset);
    }

    #[test]
    fn in_sync_needs_both_tempo_and_phase() {
        let r = deck(10.10, 124.0);
        assert!(compare(&r, &deck(2.11, 124.0)).unwrap().in_sync);
        assert!(!compare(&r, &deck(2.11, 124.3)).unwrap().in_sync, "tempo off");
        assert!(!compare(&r, &deck(2.20, 124.0)).unwrap().in_sync, "phase off");
        assert!(compare(&r, &DeckState::default()).is_none(), "no grid, no reading");
    }

    #[test]
    fn readouts_name_direction_and_size() {
        assert_eq!(ms_text(12.4), "+12 ms");
        assert_eq!(ms_text(-4.6), "−5 ms");
        assert_eq!(tempo_text(0.4), "+0.40 BPM");
        assert_eq!(tempo_text(-1.25), "−1.25 BPM");
        assert_eq!(tempo_text(0.001), "±0.00 BPM");
    }

    #[test]
    fn the_master_is_the_reference_else_the_deck_that_plays() {
        let mut s = EngineState::default();
        s.decks[0] = deck(1.0, 124.0);
        s.decks[0].playing = false;
        s.decks[1] = deck(1.0, 124.0);
        assert_eq!(reference(&s, &[0, 1]), Some(1), "the playing deck");
        s.master_deck = Some(0);
        assert_eq!(reference(&s, &[0, 1]), Some(0), "the sync master");
        // The 2-deck view on decks 3 and 2: a hidden master does not count.
        s.decks[2] = deck(1.0, 124.0);
        s.decks[2].playing = false;
        assert_eq!(reference(&s, &[2, 1]), Some(1), "the playing shown deck");
        s.decks[1].playing = false;
        assert_eq!(reference(&s, &[2, 1]), Some(2), "the first shown deck");
    }
}
