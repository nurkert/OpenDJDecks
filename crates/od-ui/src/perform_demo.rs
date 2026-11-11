//! Documentation screenshots for roadmap 0.9.0 block G
//! (`OPENDJDECKS_DEMO_PROOF`, comma separated), staged on the library in
//! use with the deck loaded by `OPENDJDECKS_AUTOLOAD`. They write hot
//! cues, a drop marker and the set's planned start and notes into that
//! library, so use them on the demo library only.
//!
//! - `perform-cue-edit`: deck 1 in CUE EDIT with named cues, one a gate,
//!   the drop marked, the cue panel beside the list.
//! - `perform-phrase`: deck 1 in the PHRASE pad mode playing up to its
//!   marked drop.
//! - `perform-plan`: the set "Saturday Set" as the plan, deck 1 playing a
//!   track of it a little behind the planned start, with a transition note.

use crate::app::App;
use od_controller::PadMode;
use od_core::{Cue, CueTrigger};
use od_engine::DeckCommand;
use slint::ComponentHandle;
use std::rc::Rc;
use std::time::Duration;

pub(crate) fn run(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let parts: Vec<String> = v.split(',').map(str::to_string).collect();
    if !parts.iter().any(|p| p.starts_with("perform-")) {
        return;
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(Duration::from_secs(6), move || {
        let Some(a) = weak.upgrade() else { return };
        a.ui.invoke_set_mode("perform".into());
        for p in &parts {
            match p.as_str() {
                "perform-cue-edit" => cue_edit(&a),
                "perform-phrase" => phrase(&a),
                "perform-plan" => plan(&a),
                _ => {}
            }
        }
    });
}

/// Deck 1's track, its grid's bar length and a downbeat near `t` seconds.
fn deck1(app: &App, t: f64) -> Option<(od_core::TrackId, f64)> {
    let dk = app.decks.borrow();
    let m = dk.loaded(0)?;
    Some((m.id, crate::deck_craft::nearest_downbeat(&m.grid, t)))
}

/// Named hot cues A, C (a gate) and E (a latch), and the drop at about a
/// third of the track.
fn stage_cues(app: &App) -> Option<f64> {
    let (id, _) = deck1(app, 0.0)?;
    let len = app.state.borrow().lib.track(id).ok()?.duration;
    let at = |f: f64| deck1(app, len * f).map_or(len * f, |x| x.1);
    let mut a = Cue::hot(0, at(0.05));
    a.label = "Intro".into();
    let mut c = Cue::hot(2, at(0.25));
    c.label = "Vocal in".into();
    c.trigger = CueTrigger::Gate;
    c.color = crate::player::CUE_PALETTE.get(4).copied();
    let mut e = Cue::hot(4, at(0.55));
    e.label = "Break".into();
    e.trigger = CueTrigger::Latch;
    // Eight bars into the first chorus: the part before becomes the build-up.
    let drop = {
        let dk = app.decks.borrow();
        let m = dk.loaded(0)?;
        let ch = m.phrases.iter().find(|p| p.kind == od_core::PhraseKind::Chorus)?;
        let beat = (ch.start_beat + 32).min(ch.end_beat.saturating_sub(1)) as usize;
        m.grid.beats.get(beat).map(|b| b.time)?
    };
    {
        let mut st = app.state.borrow_mut();
        st.lib.set_cues(id, vec![a, c, e]).ok()?;
        st.lib.set_drop_marker(id, Some(drop)).ok()?;
    }
    // The deck reads them as a fresh load would.
    crate::decks::refresh_cues(app, id);
    crate::deck_craft::refresh_drop(app, id);
    for (slot, cue) in [(0, 0.05), (2, 0.25), (4, 0.55)] {
        crate::engine::deck_cmd(app, 0, DeckCommand::SetHotCueAt { slot, start: at(cue), loop_end: None });
    }
    crate::engine::deck_cmd(app, 0, DeckCommand::SetHotCueTrigger { slot: 2, trigger: CueTrigger::Gate });
    crate::engine::deck_cmd(app, 0, DeckCommand::SetHotCueTrigger { slot: 4, trigger: CueTrigger::Latch });
    Some(drop)
}

fn cue_edit(app: &Rc<App>) {
    if stage_cues(app).is_none() {
        return;
    }
    crate::decks::set_pad_mode(app, 0, PadMode::CueEdit);
    crate::deck_craft::select_pad(app, 0, 2);
}

fn phrase(app: &Rc<App>) {
    let Some(drop) = stage_cues(app) else { return };
    crate::decks::set_pad_mode(app, 0, PadMode::Phrase);
    crate::engine::deck_cmd(app, 0, DeckCommand::Seek { seconds: (drop - 6.0).max(0.0) });
    crate::engine::deck_cmd(app, 0, DeckCommand::Play);
    // The picture is taken before the drop passes the playhead.
    let w = Rc::downgrade(app);
    slint::Timer::single_shot(Duration::from_secs(3), move || {
        if let Some(a) = w.upgrade() {
            crate::engine::deck_cmd(&a, 0, DeckCommand::Pause);
        }
    });
}

fn plan(app: &Rc<App>) {
    let node = {
        let st = app.state.borrow();
        st.lib
            .all_nodes()
            .ok()
            .and_then(|n| n.into_iter().find(|n| n.kind == od_library::NodeKind::Set).map(|n| n.id))
    };
    let Some(node) = node else { return };
    let Some((track, _)) = deck1(app, 0.0) else { return };
    // The set is a few minutes late: the planned start is the time at
    // which the track on deck 1 should have started, three minutes ago.
    let (offset, pos) = {
        let st = app.state.borrow();
        let entries = st.lib.entries(node).unwrap_or_default();
        let Some(pos) = entries.iter().position(|e| e.track == track) else { return };
        let off: f64 = entries[..pos]
            .iter()
            .filter_map(|e| st.lib.track(e.track).ok())
            .map(|t| t.duration)
            .sum();
        (off, pos)
    };
    let now: Option<String> = app
        .state
        .borrow()
        .lib
        .conn()
        .query_row(
            "SELECT strftime('%H:%M', 'now', 'localtime', ?1)",
            [format!("-{} seconds", (offset + 30.0 + 180.0).round() as i64)],
            |r| r.get(0),
        )
        .ok();
    {
        let mut st = app.state.borrow_mut();
        if let Ok(n) = st.lib.node(node) {
            let mut meta = n.set_meta.unwrap_or_default();
            meta.planned_start = now;
            let _ = st.lib.set_set_meta(node, meta);
        }
        let _ = st.lib.set_entry_note(node, pos, "Ride the pad under the intro, swap the bass on the bar");
    }
    app.ui.invoke_select_section("library".into());
    app.ui.invoke_select_node(node.0 as i32);
    crate::engine::deck_cmd(app, 0, DeckCommand::Seek { seconds: 30.0 });
    crate::engine::deck_cmd(app, 0, DeckCommand::Play);
}
