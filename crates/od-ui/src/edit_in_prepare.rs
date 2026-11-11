//! EDIT IN PREPARE: from a Perform deck straight to its track in the
//! Prepare player, with the editor for what is wrong open, and back to
//! where you were. Plus quick fixes on the deck itself, without leaving
//! Perform.
//!
//! The EDIT key in the deck header (and the deck's right-click menu on the
//! waveform, the phrase strip and the key) puts the deck's track in the
//! player at the deck's position, with the PHRASE tab (or the one asked
//! for: grid, cues, key, gain, track info, stems) open. Nothing is loaded
//! or reloaded: the decks keep playing, Prepare only reads the same
//! analysis from the library. Every edit there reaches the loaded decks at
//! once ([`crate::decks::follow_library`]). BACK TO PERFORM in the top bar,
//! `Esc`, the mode shortcut or the PERFORM tab return to the same decks and
//! view state; the Prepare layout, section and info panel are put back as
//! they were.
//!
//! Quick fixes (each one undo step): the downbeat one beat earlier or later,
//! the phrase boundary nearest the playhead one bar earlier or later, and
//! the likely other key of an unsure analysis ([62]).

use crate::app::{App, handler};
use crate::engine;
use od_core::{Beatgrid, Phrase, TrackId};
use slint::{ComponentHandle, SharedString};
use std::cell::RefCell;
use std::rc::Rc;

/// What can be opened, in menu order; the first is the default.
pub(crate) const TARGETS: [&str; 7] = ["phrases", "grid", "cues", "key", "gain", "info", "stems"];

/// Beats in a bar for the phrase quick fix (phrases start on bars).
const BAR: u32 = 4;

/// Where Prepare was opened from, and what to put back on the way out.
#[derive(Clone, Debug)]
struct Origin {
    layout: String,
    section: String,
    info_open: bool,
}

thread_local! {
    static ORIGIN: RefCell<Option<Origin>> = const { RefCell::new(None) };
}

/// Prepare was opened from Perform (BACK TO PERFORM shows).
pub(crate) fn from_perform() -> bool {
    ORIGIN.with(|o| o.borrow().is_some())
}

/// The target the EDIT key opens for deck `d`: the beatgrid while the
/// deck's GRID panel is open, else the phrases.
pub(crate) fn default_target(app: &App, d: usize) -> &'static str {
    if app.ui.global::<crate::DeckGrid>().invoke_is_open(d as i32 + 1) {
        "grid"
    } else {
        TARGETS[0]
    }
}

/// The deck's track if it can be edited here (loaded, not from a stick).
fn editable(app: &App, d: usize) -> Option<TrackId> {
    let dk = app.decks.borrow();
    let Some(id) = dk.loaded_id(d) else {
        drop(dk);
        app.status("Load a track on the deck first");
        return None;
    };
    if dk.source_is_stick() {
        drop(dk);
        app.status("Tracks on a stick are read-only here — edit them in your library and the stick follows");
        return None;
    }
    Some(id)
}

/// Deck `d`'s (0-based) track in the Prepare player at the deck's position,
/// with `target`'s editor open.
pub(crate) fn open(app: &App, d: usize, target: &str) {
    // "" (the EDIT key): what fits the deck's view.
    let target = if target.is_empty() {
        default_target(app, d)
    } else {
        TARGETS.iter().copied().find(|t| *t == target).unwrap_or(TARGETS[0])
    };
    let Some(id) = editable(app, d) else { return };
    let ui = &app.ui;
    if ui.get_mode() == "perform" && !from_perform() {
        let origin = Origin {
            layout: ui.get_prepare_layout().to_string(),
            section: ui.get_section().to_string(),
            info_open: ui.get_info_open(),
        };
        ORIGIN.with(|o| *o.borrow_mut() = Some(origin));
    }
    // A grid session on another track ends as DONE would end it.
    if crate::grid_edit::active() && crate::grid_edit::session_track() != Some(id) {
        crate::grid_edit::done(app);
    }
    // The Prepare preview is heard on the master: it stops, so the player
    // shows the deck's track and nothing new sounds.
    if app.state.borrow().player.previewing {
        engine::deck_cmd(app, crate::decks::PREVIEW_DECK, od_engine::DeckCommand::Pause);
        app.state.borrow_mut().player.previewing = false;
        ui.set_previewing(false);
    }
    let pos = engine::snapshot(app)
        .and_then(|s| s.decks.get(d).map(|s| s.position))
        .unwrap_or(0.0);
    {
        let mut st = app.state.borrow_mut();
        st.player.current = Some(id);
        st.player.cursor = pos.max(0.0);
    }
    let layout = ui.get_prepare_layout();
    if layout != "player" && layout != "two" {
        crate::shell::set_prepare_layout(app, "player");
    }
    let tabs = ui.global::<crate::PlayerTabs>();
    tabs.set_focus(SharedString::default());
    match target {
        "phrases" => tabs.set_tool("phrase".into()),
        "grid" => tabs.set_tool("grid".into()),
        "cues" => {
            tabs.set_tool("cue".into());
            tabs.set_list("all".into());
        }
        "key" | "gain" => {
            tabs.set_list("info".into());
            tabs.set_focus(target.into());
        }
        "info" => {
            ui.invoke_select_section("library".into());
            ui.set_info_open(true);
        }
        _ => tabs.set_tool("stems".into()),
    }
    let g = ui.global::<crate::EditInPrepare>();
    g.set_from_perform(from_perform());
    g.set_deck(d as i32 + 1);
    g.set_target(target.into());
    ui.invoke_set_mode("prepare".into());
    crate::player::refresh_detail(app);
    if target == "grid" && !crate::grid_edit::active() {
        crate::grid_edit::start(app);
    }
    if from_perform() {
        app.status(od_i18n::trf(
            "Deck {} in Prepare — the decks play on; BACK TO PERFORM or Esc returns",
            &[&(d + 1)],
        ));
    }
}

/// Back to Perform as it was. Nothing when Prepare was not opened from a
/// deck (returns false).
pub(crate) fn back(app: &App) -> bool {
    if !end(app) {
        return false;
    }
    app.ui.invoke_set_mode("perform".into());
    app.status("Back in Perform");
    true
}

/// Ends the trip: an EDIT GRID session is kept (DONE), the Prepare layout,
/// section and info panel are put back. False when there was no trip.
fn end(app: &App) -> bool {
    let Some(o) = ORIGIN.with(|o| o.borrow_mut().take()) else {
        return false;
    };
    if crate::grid_edit::active() {
        crate::grid_edit::done(app);
    }
    let ui = &app.ui;
    if ui.get_prepare_layout() != o.layout.as_str() {
        ui.set_prepare_layout(o.layout.as_str().into());
        let _ = app.state.borrow_mut().lib.set_meta(crate::shell::META_LAYOUT, &o.layout);
    }
    if ui.get_info_open() != o.info_open {
        ui.set_info_open(o.info_open);
    }
    if ui.get_section() != o.section.as_str() {
        ui.invoke_select_section(o.section.as_str().into());
    }
    ui.global::<crate::PlayerTabs>().set_focus(SharedString::default());
    let g = ui.global::<crate::EditInPrepare>();
    g.set_from_perform(false);
    g.set_target(SharedString::default());
    true
}

/// The mode changed by any way (the PERFORM tab, the mode shortcut,
/// Lighting): leaving Prepare ends the trip.
pub(crate) fn mode_changed(app: &App) {
    if from_perform() && app.ui.get_mode() != "prepare" {
        end(app);
    }
}

/// Keyboard and controller: from Perform to Prepare with deck `d`'s track;
/// again (in Prepare, opened from a deck) back to Perform.
pub(crate) fn toggle(app: &App, d: usize) {
    if app.ui.get_mode() == "prepare" && from_perform() {
        back(app);
    } else {
        open(app, d, default_target(app, d));
    }
}

/// A quick fix on deck `d` (0-based) without leaving Perform (one undo
/// step each).
pub(crate) fn quick(app: &App, d: usize, action: &str) {
    let Some(id) = editable(app, d) else { return };
    match action {
        "beat-earlier" => crate::deck_grid::act(app, d, "downbeat", "-1"),
        "beat-later" => crate::deck_grid::act(app, d, "downbeat", "1"),
        "phrase-earlier" => move_phrase(app, d, id, -1),
        "phrase-later" => move_phrase(app, d, id, 1),
        "key-alt" => take_alt_key(app, id),
        _ => {}
    }
}

/// The phrase boundary nearest `pos` (seconds) moved by `bars` bars, as
/// stored phrases (pure; `None` when there is no boundary, no grid, or the
/// move would pass a neighbouring boundary).
pub(crate) fn shifted_boundary(
    phrases: &[Phrase],
    grid: &Beatgrid,
    pos: f64,
    bars: i32,
) -> Option<Vec<Phrase>> {
    if grid.beats.is_empty() || phrases.is_empty() {
        return None;
    }
    let beat = grid.beat_position(pos)?.round().max(0.0) as u32;
    let mut bounds: Vec<u32> = phrases.iter().flat_map(|p| [p.start_beat, p.end_beat]).collect();
    bounds.sort_unstable();
    bounds.dedup();
    let from = *bounds.iter().min_by_key(|b| b.abs_diff(beat))?;
    let to = from as i64 + bars as i64 * BAR as i64;
    if to < 0 || to > grid.beats.len() as i64 {
        return None;
    }
    let moved = crate::phrase_edit::move_edge(phrases, from, to as u32)?;
    let normalized = od_core::normalize_phrases(&moved);
    (normalized != od_core::normalize_phrases(phrases)).then_some(normalized)
}

fn move_phrase(app: &App, d: usize, id: TrackId, bars: i32) {
    let pos = engine::snapshot(app)
        .and_then(|s| s.decks.get(d).map(|s| s.position))
        .unwrap_or(0.0);
    let (phrases, grid) = {
        let st = app.state.borrow();
        (
            st.lib.phrases(id).unwrap_or_default(),
            st.lib.beatgrid(id).ok().flatten().unwrap_or_default(),
        )
    };
    if phrases.is_empty() {
        app.status("This track has no phrases");
        return;
    }
    let Some(next) = shifted_boundary(&phrases, &grid, pos, bars) else {
        app.status("The boundary cannot move past the next one");
        return;
    };
    let r = app.state.borrow_mut().lib.set_phrases(id, &next);
    if app.report(r).is_some() {
        app.after_edit(if bars < 0 {
            "Phrase boundary one bar earlier"
        } else {
            "Phrase boundary one bar later"
        });
    }
}

fn take_alt_key(app: &App, id: TrackId) {
    let alt = app
        .state
        .borrow()
        .lib
        .analysis(id)
        .ok()
        .and_then(|a| a.confidence)
        .and_then(|c| c.key_alt);
    let Some(alt) = alt else {
        app.status("The analysis has no other likely key for this track");
        return;
    };
    let r = app.state.borrow_mut().lib.set_key(id, Some(alt));
    if app.report(r).is_some() {
        let notation = app.state.borrow().settings.key_notation();
        app.after_edit(&od_i18n::trf("Key set to {}", &[&alt.format(notation)]));
    }
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::EditInPrepare>();
    g.on_open(handler!(app, |a, d: i32, target: SharedString| {
        open(&a, (d.max(1) - 1) as usize, target.as_str())
    }));
    g.on_quick(handler!(app, |a, d: i32, action: SharedString| {
        quick(&a, (d.max(1) - 1) as usize, action.as_str())
    }));
    g.on_back(handler!(app, |a| {
        back(&a);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::PhraseKind;

    fn grid() -> Beatgrid {
        // 120 BPM: a beat every 0.5 s, 256 beats.
        Beatgrid::constant(120.0, 0.0, 1, 128.0)
    }

    #[test]
    fn the_boundary_under_the_playhead_moves_one_bar() {
        let phrases = vec![
            Phrase::new(0, 64, PhraseKind::Intro),
            Phrase::new(64, 128, PhraseKind::Verse),
            Phrase::new(128, 256, PhraseKind::Chorus),
        ];
        // Playhead at beat 66: the boundary at 64 is nearest.
        let later = shifted_boundary(&phrases, &grid(), 33.0, 1).unwrap();
        assert_eq!((later[0].end_beat, later[1].start_beat), (68, 68));
        assert_eq!(later[2].start_beat, 128, "the others stay");
        let earlier = shifted_boundary(&phrases, &grid(), 33.0, -1).unwrap();
        assert_eq!((earlier[0].end_beat, earlier[1].start_beat), (60, 60));
        // Near the end of the track: no boundary past the last beat.
        assert!(shifted_boundary(&phrases, &grid(), 127.0, 1).is_none());
        assert!(shifted_boundary(&[], &grid(), 33.0, 1).is_none());
    }
}
