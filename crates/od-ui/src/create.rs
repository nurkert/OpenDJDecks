//! CREATE (roadmap [70]): the fourth work mode, for what DJs make before a
//! set: a melody from one note (MELODY), a small drum machine (DRUMS) and
//! which notes fit (THEORY). The concept is in roadmap 0.10.0.
//!
//! This is the shell: the three views with what comes where, the RESULT
//! bar, and a slim deck strip so the decks stay in view. The decks keep
//! playing: a mode switch only changes what is shown, and this module only
//! reads the engine's deck state for the strip.

use crate::app::App;
use od_engine::{DeckState, EngineState};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

/// The views of CREATE, in the order of their tabs.
pub(crate) const VIEWS: [&str; 3] = ["melody", "drums", "theory"];

/// One deck of the strip from its engine state, channel level and key.
pub(crate) fn strip_row(i: usize, s: &DeckState, level: f32, key: String) -> crate::CreateDeck {
    let loaded = s.track.is_some();
    // Where in the bar: the beat of the bar (1..=4) and how far into it.
    let bar = if loaded && s.beat_in_bar > 0 {
        ((f64::from(s.beat_in_bar) - 1.0 + s.beat.rem_euclid(1.0)) / 4.0) as f32
    } else {
        0.0
    };
    crate::CreateDeck {
        deck: i as i32 + 1,
        loaded,
        playing: loaded && s.playing,
        bpm: if loaded { crate::format::bpm(Some(s.bpm), 1) } else { String::new() }.into(),
        key: key.into(),
        progress: if loaded && s.duration > 0.0 { (s.position / s.duration).clamp(0.0, 1.0) as f32 } else { 0.0 },
        bar,
        level: if loaded { level.clamp(0.0, 1.0) } else { 0.0 },
    }
}

/// Frame tick while CREATE is shown: the deck strip (and the decks' keys
/// for THEORY). Before audio runs the decks show empty.
pub(crate) fn frame(app: &App, state: Option<&EngineState>) {
    let count = app.decks.borrow().deck_count;
    let rows: Vec<crate::CreateDeck> = (0..count)
        .map(|i| match state.filter(|s| i < s.decks.len()) {
            Some(s) => {
                let d = &s.decks[i];
                strip_row(i, d, s.channels[i].meter.peak, crate::decks::key_text(app, i, d.track))
            }
            None => strip_row(i, &DeckState::default(), 0.0, String::new()),
        })
        .collect();
    let g = app.ui.global::<crate::Create>();
    // Same row count: update in place (no relayout every frame).
    let model = g.get_decks();
    if model.row_count() == rows.len() {
        for (i, row) in rows.into_iter().enumerate() {
            if model.row_data(i).as_ref() != Some(&row) {
                model.set_row_data(i, row);
            }
        }
    } else {
        g.set_decks(ModelRc::new(VecModel::from(rows)));
    }
}

/// Sets the view (MELODY, DRUMS or THEORY); unknown names are ignored.
pub(crate) fn set_view(app: &App, view: &str) {
    if VIEWS.contains(&view) {
        app.ui.global::<crate::Create>().set_view(view.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_row_reads_play_state_tempo_bar_and_level() {
        let empty = strip_row(0, &DeckState::default(), 0.7, String::new());
        assert!(!empty.loaded && !empty.playing);
        assert_eq!(empty.bpm, "");
        assert_eq!((empty.progress, empty.bar, empty.level), (0.0, 0.0, 0.0));

        let s = DeckState {
            track: Some(od_core::TrackId(7)),
            duration: 200.0,
            position: 50.0,
            playing: true,
            bpm: 124.04,
            beat: 33.5,
            beat_in_bar: 3,
            ..Default::default()
        };
        let r = strip_row(1, &s, 1.4, "8A".into());
        assert_eq!(r.deck, 2);
        assert!(r.loaded && r.playing);
        assert_eq!(r.bpm, "124.0");
        assert_eq!(r.key, "8A");
        assert!((r.progress - 0.25).abs() < 1e-6);
        // Beat 3 of the bar, half way through it: 2.5 of 4 beats.
        assert!((r.bar - 0.625).abs() < 1e-6, "{}", r.bar);
        assert_eq!(r.level, 1.0, "clamped");
    }

    #[test]
    fn no_grid_no_bar_position() {
        let s = DeckState { track: Some(od_core::TrackId(1)), beat: 3.2, beat_in_bar: 0, ..Default::default() };
        assert_eq!(strip_row(0, &s, 0.0, String::new()).bar, 0.0);
    }
}
