//! [60] Stems on the deck: solo and keep-only-this, the state each stem key
//! shows, and the warning when a track without stems comes in where stems
//! were in use.
//!
//! Solo is a toggle (a right click on a stem key, or held on a controller):
//! the other parts go quiet and come back as they were when solo ends. Keep
//! only this (⇧-click) mutes the other parts for good, as if each had been
//! clicked.

use crate::app::App;
use crate::{decks, engine};
use od_engine::{DeckCommand, DeckState, Stem};

/// The four parts, in the order of [`Stem::index`].
const PARTS: [Stem; 4] = [Stem::Vocals, Stem::Drums, Stem::Bass, Stem::Inst];

/// A running solo: the soloed part and the levels to return to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Solo {
    pub stem: Stem,
    pub saved: [f32; 4],
}

/// Levels while `stem` is soloed: it at full level (or its own level when
/// that is up), the others silent.
pub(crate) fn solo_levels(current: [f32; 4], stem: Stem) -> [f32; 4] {
    std::array::from_fn(|i| {
        if i == stem.index() {
            if current[i] > 0.01 { current[i] } else { 1.0 }
        } else {
            0.0
        }
    })
}

/// Commands that move `from` to `to` (only the parts that change).
fn level_commands(from: [f32; 4], to: [f32; 4]) -> Vec<DeckCommand> {
    PARTS
        .iter()
        .filter(|p| (from[p.index()] - to[p.index()]).abs() > 1e-4)
        .map(|&p| DeckCommand::SetStemLevel {
            stem: p,
            level: to[p.index()],
        })
        .collect()
}

/// The deck's parts are there to solo (asks for them otherwise, like a
/// press on a part key).
fn ready(app: &App, d: usize) -> Option<DeckState> {
    engine::ensure_engine(app);
    let s = decks::deck_state(app, d)?;
    if !s.stems_available {
        crate::stems_model::part_pressed(app, d);
        return None;
    }
    Some(s)
}

/// Solo `stem` on deck `d` (`on` false ends a solo of it).
pub(crate) fn set_solo(app: &App, d: usize, stem: Stem, on: bool) {
    let Some(s) = ready(app, d) else { return };
    let current = app.decks.borrow().stem_solo[d];
    let cmds = match (current, on) {
        // Ending: back to the levels from before.
        (Some(solo), false) if solo.stem == stem => {
            app.decks.borrow_mut().stem_solo[d] = None;
            level_commands(s.stem_levels, solo.saved)
        }
        (_, false) => return,
        // Another part while soloed: the saved levels stay.
        (Some(solo), true) => {
            app.decks.borrow_mut().stem_solo[d] = Some(Solo { stem, ..solo });
            level_commands(s.stem_levels, solo_levels(solo.saved, stem))
        }
        (None, true) => {
            app.decks.borrow_mut().stem_solo[d] = Some(Solo {
                stem,
                saved: s.stem_levels,
            });
            level_commands(s.stem_levels, solo_levels(s.stem_levels, stem))
        }
    };
    for c in cmds {
        engine::deck_cmd(app, d, c);
    }
}

/// Right click on a stem key: solo it, or end its solo.
pub(crate) fn toggle_solo(app: &App, d: usize, stem: Stem) {
    let on = app.decks.borrow().stem_solo[d].is_none_or(|s| s.stem != stem);
    set_solo(app, d, stem, on);
}

/// Keep only `stem`: the other parts muted, and they stay muted (ends a solo).
pub(crate) fn keep_only(app: &App, d: usize, stem: Stem) {
    let Some(s) = ready(app, d) else { return };
    let solo = app.decks.borrow_mut().stem_solo[d].take();
    let base = solo.map_or(s.stem_levels, |x| x.saved);
    for c in level_commands(s.stem_levels, solo_levels(base, stem)) {
        engine::deck_cmd(app, d, c);
    }
    app.status(od_i18n::trf(
        "Deck {}: only {} plays",
        &[&(d + 1), &od_i18n::tr(part_name(stem))],
    ));
}

/// A click mutes / unmutes as before; during a solo it ends the solo first
/// (the levels from before come back, then the click applies).
pub(crate) fn end_solo(app: &App, d: usize) {
    let solo = app.decks.borrow_mut().stem_solo[d].take();
    if let (Some(solo), Some(s)) = (solo, decks::deck_state(app, d)) {
        for c in level_commands(s.stem_levels, solo.saved) {
            engine::deck_cmd(app, d, c);
        }
    }
}

/// UI id of a part ("vocal", "drums", "bass", "inst").
pub(crate) fn part_id(s: Stem) -> &'static str {
    match s {
        Stem::Vocals => "vocal",
        Stem::Drums => "drums",
        Stem::Bass => "bass",
        Stem::Inst => "inst",
    }
}

fn part_name(s: Stem) -> &'static str {
    match s {
        Stem::Vocals => "VOCAL",
        Stem::Drums => "DRUMS",
        Stem::Bass => "BASS",
        Stem::Inst => "INST",
    }
}

/// What the stem keys of a deck show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyState {
    /// No track.
    Empty,
    /// The parts are in: the keys work.
    Ready,
    /// Being separated now (the keys fill as a progress bar).
    Separating,
    /// Waiting in the stems-ahead queue.
    Queued,
    /// Not separated.
    None,
}

impl KeyState {
    pub fn id(self) -> &'static str {
        match self {
            KeyState::Empty => "",
            KeyState::Ready => "ready",
            KeyState::Separating => "separating",
            KeyState::Queued => "queued",
            KeyState::None => "none",
        }
    }
}

/// The state of a deck's stem keys (pure).
pub(crate) fn key_state(loaded: bool, available: bool, separating: bool, queued: bool) -> KeyState {
    match () {
        _ if !loaded => KeyState::Empty,
        _ if available => KeyState::Ready,
        _ if separating => KeyState::Separating,
        _ if queued => KeyState::Queued,
        _ => KeyState::None,
    }
}

/// Stems are expected on a deck when the track it replaces had them, or
/// another deck plays with them: a track without parts then gets a warning.
pub(crate) fn expected(replaced_had_stems: bool, others_with_stems: bool) -> bool {
    replaced_had_stems || others_with_stems
}

/// Documentation screenshots (`OPENDJDECKS_DEMO_PROOF=stems-ahead`, with
/// two tracks without stems on the decks): the library's STEMS column on,
/// the rest of the list shown as queued (staged, the batch separates
/// nothing), deck 1 warning that stems were expected (or separating, when
/// the model is installed) and deck 2 queued.
pub(crate) fn demo(app: &std::rc::Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    if !v.split(',').any(|p| p == "stems-ahead") {
        return;
    }
    let weak = std::rc::Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(5), move || {
        let Some(a) = weak.upgrade() else { return };
        {
            let mut st = a.state.borrow_mut();
            let mut cols = crate::browser::sanitize_columns(&st.settings.browser.columns);
            if !cols.iter().any(|c| c == "stems") {
                cols = crate::browser::toggle_column(&cols, "stems");
            }
            // Room for the column: genre and preview step aside (in memory
            // only; the demo library keeps its own choice).
            cols.retain(|c| c != "genre" && c != "preview");
            st.settings.browser.columns = cols;
        }
        // Tracks with stems in the cache read READY by themselves; the
        // others (deck 2's too) show as queued.
        let cache = a.decks.borrow().stem_cache();
        let on_decks = a.decks.borrow().loaded_ids();
        let rest: Vec<_> = a
            .state
            .borrow()
            .browser
            .rows
            .iter()
            .filter(|t| !on_decks.contains(&t.id) && !crate::stems_batch::cached(&cache, &t.content_hash))
            .map(|t| t.id)
            .collect();
        let queued: Vec<_> = on_decks.get(1).copied().into_iter().chain(rest).collect();
        crate::stems_batch::stage(&[], None, &queued);
        a.decks.borrow_mut().stems_expected[0] = true;
        crate::browser::apply_columns(&a);
        crate::browser::refresh_tracks(&a);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_silences_the_others_and_comes_back() {
        let before = [1.0, 0.0, 0.6, 1.0];
        let solo = solo_levels(before, Stem::Bass);
        assert_eq!(solo, [0.0, 0.0, 0.6, 0.0], "bass keeps its own level");
        // A muted part soloed plays at full level.
        assert_eq!(solo_levels(before, Stem::Drums), [0.0, 1.0, 0.0, 0.0]);
        let on = level_commands(before, solo);
        assert_eq!(on.len(), 2, "vocals and inst go quiet: {on:?}");
        let back = level_commands(solo, before);
        assert_eq!(back.len(), 2);
        assert!(level_commands(before, before).is_empty());
    }

    #[test]
    fn key_states() {
        assert_eq!(key_state(false, false, false, false), KeyState::Empty);
        assert_eq!(key_state(true, true, true, true), KeyState::Ready);
        assert_eq!(key_state(true, false, true, true), KeyState::Separating);
        assert_eq!(key_state(true, false, false, true), KeyState::Queued);
        assert_eq!(key_state(true, false, false, false).id(), "none");
        assert!(expected(true, false) && expected(false, true) && !expected(false, false));
    }
}
