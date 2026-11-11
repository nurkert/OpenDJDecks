//! Deck craft of roadmap 0.9.0 block G: reverse and cut-in on the deck,
//! phrase jumps that land in time, the drop you mark on a track, and hot
//! cue edits without leaving Perform (name, colour, move to the playhead,
//! delete, and how the pad plays the cue).
//!
//! The jump arithmetic is pure and works in grid beats, like
//! [`crate::phrase_mix`].

use crate::app::{App, handler};
use crate::{decks, engine};
use od_core::{CueKind, CueTrigger, Phrase, PhraseKind, TrackId};
use od_engine::{DeckCommand, DeckState};
use slint::ComponentHandle;
use std::rc::Rc;

/// Beats per bar.
const BAR: f64 = 4.0;

/// The grid beat of the next downbeat after `beat` (`beat_in_bar` is the
/// bar position 1..=4 of the current beat; `None` without a grid).
pub(crate) fn next_downbeat(beat: f64, beat_in_bar: u8) -> Option<f64> {
    if !(1..=4).contains(&beat_in_bar) {
        return None;
    }
    Some(beat.floor() + (5 - beat_in_bar) as f64)
}

/// The whole-beat jump that puts grid beat `target` where the next
/// downbeat would be: the playhead keeps its place in the bar, so `target`
/// arrives on the next downbeat, in time with the other decks. `None` when
/// there is no grid, nothing to move, or the jump would leave the track.
pub(crate) fn landing_jump(beat: f64, beat_in_bar: u8, target: f64) -> Option<f64> {
    let nd = next_downbeat(beat, beat_in_bar)?;
    let jump = (target - nd).round();
    (jump.abs() >= 1.0 && beat + jump >= 0.0).then_some(jump)
}

/// Phrase starts (and the end of the last phrase), sorted; without phrases
/// the grid's 16-bar marks up to `beat` + 4 phrases.
fn starts(phrases: &[Phrase], beat: f64) -> Vec<f64> {
    if phrases.is_empty() {
        let p = crate::phrase_mix::FALLBACK_PHRASE;
        let last = ((beat / p).floor() + 4.0) * p;
        return (0..=(last / p) as usize).map(|i| i as f64 * p).collect();
    }
    let mut b: Vec<f64> = phrases.iter().map(|p| p.start_beat as f64).collect();
    if let Some(end) = phrases.iter().map(|p| p.end_beat).max() {
        b.push(end as f64);
    }
    b.sort_by(f64::total_cmp);
    b.dedup();
    b
}

/// The phrase boundary a phrase jump goes to: forward, the first phrase
/// start after the next downbeat; back, the start of the current phrase,
/// or of the one before when the next downbeat is still in the current
/// phrase's first bar.
pub(crate) fn phrase_target(phrases: &[Phrase], beat: f64, beat_in_bar: u8, forward: bool) -> Option<f64> {
    let nd = next_downbeat(beat, beat_in_bar)?;
    let b = starts(phrases, beat);
    if forward {
        b.into_iter().find(|&x| x > nd + 0.5)
    } else {
        let cur = b.iter().rposition(|&x| x <= beat + 1e-6)?;
        if nd - b[cur] <= BAR + 0.5 {
            cur.checked_sub(1).map(|i| b[i])
        } else {
            Some(b[cur])
        }
    }
}

/// The next phrase of a kind after the next downbeat: build-up (Up), drop
/// (a Chorus after something else), breakdown (Down or Bridge) or outro.
pub(crate) fn kind_target(phrases: &[Phrase], beat: f64, beat_in_bar: u8, kind: PhraseKind) -> Option<f64> {
    let nd = next_downbeat(beat, beat_in_bar)?;
    let mut sorted = phrases.to_vec();
    sorted.sort_by_key(|p| p.start_beat);
    let mut prev = None;
    for p in &sorted {
        let hit = match kind {
            PhraseKind::Chorus => p.kind == PhraseKind::Chorus && prev != Some(PhraseKind::Chorus),
            PhraseKind::Down => matches!(p.kind, PhraseKind::Down | PhraseKind::Bridge),
            k => p.kind == k,
        };
        if hit && p.start_beat as f64 > nd + 0.5 {
            return Some(p.start_beat as f64);
        }
        prev = Some(p.kind);
    }
    None
}

/// The phrase kinds of the PHRASE pads E–H.
pub(crate) const KIND_PADS: [PhraseKind; 4] = [
    PhraseKind::Up,
    PhraseKind::Chorus,
    PhraseKind::Down,
    PhraseKind::Outro,
];

/// The legend of a PHRASE pad E–H.
pub(crate) fn kind_pad_label(k: PhraseKind) -> &'static str {
    match k {
        PhraseKind::Up => "BUILD",
        PhraseKind::Chorus => "DROP",
        PhraseKind::Down => "BREAK",
        _ => "OUTRO",
    }
}

/// What a PHRASE pad jumps to (grid beat), for pads A–C and E–H; D marks
/// the drop and is handled by [`toggle_drop`].
pub(crate) fn pad_target(
    phrases: &[Phrase],
    drop_beat: Option<f64>,
    s: &DeckState,
    pad: usize,
) -> Option<f64> {
    match pad {
        0 => phrase_target(phrases, s.beat, s.beat_in_bar, false),
        1 => phrase_target(phrases, s.beat, s.beat_in_bar, true),
        2 => drop_beat,
        4..=7 => kind_target(phrases, s.beat, s.beat_in_bar, KIND_PADS[pad - 4]),
        _ => None,
    }
}

/// The drop's grid beat for a drop at `at` seconds.
pub(crate) fn drop_beat(grid: &od_core::Beatgrid, at: Option<f64>) -> Option<f64> {
    at.and_then(|t| grid.nearest_beat(t)).map(|b| b as f64)
}

/// The downbeat (seconds) nearest `time`, or `time` without a grid.
pub(crate) fn nearest_downbeat(grid: &od_core::Beatgrid, time: f64) -> f64 {
    grid.beats
        .iter()
        .filter(|b| b.bar_position == 1)
        .min_by(|a, b| (a.time - time).abs().total_cmp(&(b.time - time).abs()))
        .map_or(time, |b| b.time)
}

/// Where a press of the drop key leaves the drop: cleared when the
/// playhead is within a bar of the marked drop, else marked on the
/// downbeat nearest the playhead.
pub(crate) fn next_drop(grid: &od_core::Beatgrid, current: Option<f64>, position: f64, bpm: f64) -> Option<f64> {
    let bar = if bpm > 1.0 { 240.0 / bpm } else { 2.0 };
    let at = nearest_downbeat(grid, position);
    match current {
        Some(c) if (c - at).abs() < bar * 0.75 || (c - position).abs() < bar => None,
        _ => Some(at),
    }
}

// ── App side ─────────────────────────────────────────────────────────────

/// REV on deck `d` (0-based): on / off.
pub(crate) fn toggle_reverse(app: &App, d: usize) {
    engine::ensure_engine(app);
    let on = decks::deck_state(app, d).is_some_and(|s| s.reverse);
    engine::deck_cmd(app, d, DeckCommand::SetReverse(!on));
}

/// Jumps deck `d` by `beats` so that `target` lands on the next downbeat;
/// says why when it cannot.
fn land(app: &App, d: usize, target: Option<f64>, what: &str) {
    let Some(s) = decks::deck_state(app, d).filter(|s| s.track.is_some()) else {
        return;
    };
    if s.beat_in_bar == 0 {
        app.status("No beatgrid: phrase jumps need the track's grid");
        return;
    }
    match target.and_then(|t| landing_jump(s.beat, s.beat_in_bar, t)) {
        Some(beats) => engine::deck_cmd(app, d, DeckCommand::BeatJump { beats }),
        None => app.status(od_i18n::trf("No {} to jump to", &[&od_i18n::tr(what)]).to_string()),
    }
}

/// Phrase jump on deck `d`: to the next or previous phrase, landing on the
/// next downbeat.
pub(crate) fn phrase_jump(app: &App, d: usize, forward: bool) {
    engine::ensure_engine(app);
    let Some(s) = decks::deck_state(app, d) else { return };
    let target = {
        let dk = app.decks.borrow();
        let phrases = dk.loaded(d).map(|m| m.phrases.as_slice()).unwrap_or(&[]);
        phrase_target(phrases, s.beat, s.beat_in_bar, forward)
    };
    land(app, d, target, "phrase");
}

/// PHRASE pad `pad` of deck `d` pressed.
pub(crate) fn phrase_pad(app: &App, d: usize, pad: usize) {
    engine::ensure_engine(app);
    if pad == 3 {
        toggle_drop(app, d);
        return;
    }
    let Some(s) = decks::deck_state(app, d) else { return };
    let target = {
        let dk = app.decks.borrow();
        let m = dk.loaded(d);
        let phrases = m.map(|m| m.phrases.as_slice()).unwrap_or(&[]);
        pad_target(phrases, m.and_then(|m| m.drop_beat()), &s, pad)
    };
    let what = match pad {
        0 | 1 => "phrase",
        2 => "marked drop",
        4 => "build-up",
        5 => "drop",
        6 => "breakdown",
        _ => "outro",
    };
    land(app, d, target, what);
}

/// Marks the drop of deck `d`'s track at the downbeat nearest the
/// playhead, or clears it when the playhead is at the marked drop. The
/// drop is a library edit (one undo step); the phrases of every deck and
/// the light show follow at once.
pub(crate) fn toggle_drop(app: &App, d: usize) {
    if app.guard_read_only() || app.decks.borrow().loaded_from_stick {
        return;
    }
    let Some(s) = decks::deck_state(app, d) else { return };
    let (id, grid) = {
        let dk = app.decks.borrow();
        let Some(m) = dk.loaded(d) else { return };
        (m.id, m.grid.clone())
    };
    let current = app.state.borrow().lib.drop_marker(id).ok().flatten();
    let next = next_drop(&grid, current, s.position, s.track_bpm);
    let r = app.state.borrow_mut().lib.set_drop_marker(id, next);
    if app.report(r).is_some() {
        refresh_drop(app, id);
        app.after_edit(if next.is_some() { "Drop marked" } else { "Drop cleared" });
    }
}

/// The drop and phrases of `track` changed: the decks showing it and the
/// light show read them again.
pub(crate) fn refresh_drop(app: &App, track: TrackId) {
    let (at, phrases) = {
        let st = app.state.borrow();
        (
            st.lib.drop_marker(track).ok().flatten(),
            st.lib.analysis(track).map(|a| a.phrases).unwrap_or_default(),
        )
    };
    decks::set_track_phrases(app, track, at, phrases);
    crate::lighting::forget_track(app, track);
}

// ── Hot cue edits from Perform ──────────────────────────────────────────

/// The cue edit panel of a deck: which deck (1-based, 0 = closed) and
/// which hot cue (library index).
#[derive(Default)]
pub(crate) struct CueEditState {
    pub deck: std::cell::Cell<usize>,
    pub slot: std::cell::Cell<usize>,
}

/// Edits hot cue `index` (library index) of the track on deck `d` with
/// `f`; false when the track has no such cue or is read-only.
fn edit_hot_cue(app: &App, d: usize, index: usize, label: &str, f: impl FnOnce(&mut Vec<od_core::Cue>, usize) -> bool) -> bool {
    if app.guard_read_only() || app.decks.borrow().loaded_from_stick {
        return false;
    }
    let Some(id) = app.decks.borrow().loaded(d).map(|m| m.id) else {
        return false;
    };
    let mut cues = app.state.borrow().lib.cues(id).unwrap_or_default();
    let Some(i) = cues.iter().position(|c| c.kind == CueKind::Hot(index as u8)) else {
        app.status("This pad has no hot cue yet: press it to store one");
        return false;
    };
    if !f(&mut cues, i) {
        return false;
    }
    let r = app.state.borrow_mut().lib.set_cues(id, cues);
    if app.report(r).is_none() {
        return false;
    }
    decks::refresh_cues(app, id);
    app.after_edit(label);
    true
}

/// The engine's hot cue `index` of deck `d` follows the library (after a
/// move or a trigger change), when the cue is in the deck's active bank.
fn resend_hot_cue(app: &App, d: usize, index: usize) {
    let Some(s) = decks::deck_state(app, d) else { return };
    if index / od_engine::HOT_CUES != s.hot_cue_bank {
        return;
    }
    let slot = index % od_engine::HOT_CUES;
    let Some(id) = app.decks.borrow().loaded(d).map(|m| m.id) else { return };
    let cue = app
        .state
        .borrow()
        .lib
        .cues(id)
        .unwrap_or_default()
        .into_iter()
        .find(|c| c.kind == CueKind::Hot(index as u8));
    match cue {
        Some(c) => {
            engine::deck_cmd(
                app,
                d,
                DeckCommand::SetHotCueAt {
                    slot,
                    start: c.start,
                    loop_end: c.loop_end,
                },
            );
            engine::deck_cmd(app, d, DeckCommand::SetHotCueTrigger { slot, trigger: c.trigger });
        }
        None => engine::deck_cmd(app, d, DeckCommand::DeleteHotCue { slot }),
    }
}

/// Deck `d` (0-based) changed its pad mode: CUE EDIT opens the cue edit
/// panel on the deck's first stored hot cue of the shown bank (else pad
/// A); leaving it closes the panel.
pub(crate) fn pad_mode_changed(app: &App, d: usize, new: od_controller::PadMode) {
    if new == od_controller::PadMode::CueEdit {
        let s = decks::deck_state(app, d).unwrap_or_default();
        let first = s.hot_cues.iter().position(Option::is_some).unwrap_or(0);
        app.cue_edit.deck.set(d + 1);
        app.cue_edit.slot.set(s.hot_cue_bank * od_engine::HOT_CUES + first);
    } else if app.cue_edit.deck.get() == d + 1 {
        app.cue_edit.deck.set(0);
    }
    project(app);
}

/// A pad of deck `d` pressed in CUE EDIT: that hot cue is the one edited.
pub(crate) fn select_pad(app: &App, d: usize, pad: usize) {
    let bank = decks::deck_state(app, d).map_or(0, |s| s.hot_cue_bank);
    app.cue_edit.deck.set(d + 1);
    app.cue_edit.slot.set(bank * od_engine::HOT_CUES + pad.min(7));
    project(app);
}

/// The hot cue edited on deck `d` (library index), if its panel is open.
pub(crate) fn edited_slot(app: &App, d: usize) -> Option<usize> {
    (app.cue_edit.deck.get() == d + 1).then(|| app.cue_edit.slot.get())
}

/// The drop line of the panel: where the drop is marked, or that none is.
pub(crate) fn drop_text(at: Option<f64>) -> String {
    match at {
        Some(t) => od_i18n::trf("Drop marked at {}", &[&crate::format::deck_time(t)]).to_string(),
        None => od_i18n::tr("No drop marked").to_string(),
    }
}

/// Shows the cue edit panel's state (deck, slot, name, colour, trigger,
/// the drop).
pub(crate) fn project(app: &App) {
    let g = app.ui.global::<crate::CueEditPerform>();
    let deck = app.cue_edit.deck.get();
    g.set_deck(deck as i32);
    if deck == 0 {
        return;
    }
    let index = app.cue_edit.slot.get();
    let (cues, drop) = {
        let dk = app.decks.borrow();
        let m = dk.loaded(deck - 1);
        let cues = m
            .and_then(|m| app.state.borrow().lib.cues(m.id).ok())
            .unwrap_or_default();
        (cues, m.and_then(|m| m.drop))
    };
    let bank = index / od_engine::HOT_CUES;
    let slots: Vec<crate::PadView> = (0..od_engine::HOT_CUES)
        .map(|i| {
            let idx = bank * od_engine::HOT_CUES + i;
            let c = cues.iter().find(|c| c.kind == CueKind::Hot(idx as u8));
            crate::PadView {
                index: i as i32,
                caption: crate::pads::hot_letter(idx).into(),
                sub: c.map(|c| c.label.clone()).unwrap_or_default().into(),
                set: c.is_some(),
                lit: idx == index,
                cue_slot: i as i32,
                tone: crate::player::cue_tone(c.and_then(|c| c.color)),
                badge: c.map_or("", |c| crate::pads::trigger_badge(c.trigger)).into(),
            }
        })
        .collect();
    g.set_slots(crate::models::model(slots));
    let cue = cues.into_iter().find(|c| c.kind == CueKind::Hot(index as u8));
    g.set_slot((index % od_engine::HOT_CUES) as i32);
    g.set_letter(crate::pads::hot_letter(index).into());
    g.set_set(cue.is_some());
    g.set_name(cue.as_ref().map(|c| c.label.clone()).unwrap_or_default().into());
    g.set_color(
        cue.as_ref()
            .and_then(|c| c.color)
            .and_then(|c| crate::player::CUE_PALETTE.iter().position(|p| *p == c))
            .map_or(-1, |i| i as i32),
    );
    g.set_trigger(cue.map_or(CueTrigger::Cue, |c| c.trigger).as_str().into());
    g.set_has_drop(drop.is_some());
    g.set_drop_text(drop_text(drop).into());
}

/// Stores hot cue `index` of deck `d` at `at` seconds (a pad without a cue).
fn store_hot_cue(app: &App, d: usize, index: usize, at: f64) {
    if app.guard_read_only() || app.decks.borrow().loaded_from_stick {
        return;
    }
    let Some(id) = app.decks.borrow().loaded(d).map(|m| m.id) else { return };
    let cues = app.state.borrow().lib.cues(id).unwrap_or_default();
    let next = decks::apply_hot_cue(
        cues,
        index as u8,
        Some(od_engine::HotCue {
            start: at,
            loop_end: None,
            trigger: CueTrigger::Cue,
        }),
    );
    let r = app.state.borrow_mut().lib.set_cues(id, next);
    if app.report(r).is_some() {
        decks::refresh_cues(app, id);
        resend_hot_cue(app, d, index);
        app.after_edit("Hot cue stored");
    }
}

/// The open panel's deck (0-based) and hot cue, when it is open.
fn open(app: &App) -> Option<(usize, usize)> {
    let d = app.cue_edit.deck.get();
    (d > 0).then(|| (d - 1, app.cue_edit.slot.get()))
}

/// Moves the edited hot cue to the playhead (on the beat when quantize is
/// on), or stores it there when its pad has none.
pub(crate) fn move_here(app: &App) {
    let Some((d, i)) = open(app) else { return };
    let Some(st) = engine::snapshot(app) else { return };
    let s = st.decks[d];
    if s.track.is_none() {
        return;
    }
    let grid = app.decks.borrow().loaded(d).map(|m| m.grid.clone());
    let at = match grid.filter(|g| st.quantize && !g.beats.is_empty()) {
        Some(g) => g.nearest_beat(s.position).map_or(s.position, |b| g.beats[b].time),
        None => s.position,
    };
    let exists = s.hot_cue_bank == i / od_engine::HOT_CUES && s.hot_cues[i % od_engine::HOT_CUES].is_some()
        || app.decks.borrow().loaded(d).is_some_and(|m| {
            app.state
                .borrow()
                .lib
                .cues(m.id)
                .unwrap_or_default()
                .iter()
                .any(|c| c.kind == CueKind::Hot(i as u8))
        });
    if !exists {
        store_hot_cue(app, d, i, at);
    } else if edit_hot_cue(app, d, i, "Move hot cue", |cues, k| {
        crate::player::apply_cue_edit(cues, k, crate::player::CueEditOp::MoveTo(at))
    }) {
        resend_hot_cue(app, d, i);
    }
    project(app);
}

fn wire_panel(app: &Rc<App>) {
    let g = app.ui.global::<crate::CueEditPerform>();
    g.set_palette(crate::models::model(
        crate::player::CUE_PALETTE
            .iter()
            .map(|c| crate::player::cue_tone(Some(*c)))
            .collect(),
    ));
    g.on_select(handler!(app, |a, slot: i32| {
        if let Some((d, _)) = open(&a) {
            select_pad(&a, d, slot.clamp(0, 7) as usize);
        }
    }));
    g.on_rename(handler!(app, |a, name: slint::SharedString| {
        if let Some((d, i)) = open(&a) {
            let name = name.trim().to_string();
            edit_hot_cue(&a, d, i, "Rename hot cue", |cues, k| {
                cues[k].label = name;
                true
            });
            project(&a);
        }
    }));
    g.on_recolor(handler!(app, |a, c: i32| {
        if let Some((d, i)) = open(&a) {
            edit_hot_cue(&a, d, i, "Recolour hot cue", |cues, k| {
                cues[k].color = crate::player::CUE_PALETTE.get(c.max(0) as usize).copied();
                true
            });
            project(&a);
        }
    }));
    g.on_move_here(handler!(app, |a| move_here(&a)));
    g.on_delete(handler!(app, |a| {
        let Some((d, i)) = open(&a) else { return };
        if edit_hot_cue(&a, d, i, "Delete hot cue", |cues, k| {
            cues.remove(k);
            true
        }) {
            resend_hot_cue(&a, d, i);
        }
        project(&a);
    }));
    g.on_set_trigger(handler!(app, |a, t: slint::SharedString| {
        let Some((d, i)) = open(&a) else { return };
        let Some(t) = CueTrigger::parse(t.as_str()) else { return };
        if edit_hot_cue(&a, d, i, "Hot cue pad behaviour", |cues, k| {
            cues[k].trigger = t;
            true
        }) {
            resend_hot_cue(&a, d, i);
        }
        project(&a);
    }));
    g.on_drop_mark(handler!(app, |a| {
        if let Some((d, _)) = open(&a) {
            toggle_drop(&a, d);
            project(&a);
        }
    }));
    g.on_done(handler!(app, |a| {
        if let Some((d, _)) = open(&a) {
            decks::set_pad_mode(&a, d, od_controller::PadMode::HotCue);
        }
    }));
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::DeckCraft>();
    g.on_reverse(handler!(app, |a, d: i32| toggle_reverse(&a, (d as usize).saturating_sub(1))));
    g.on_cut_in(handler!(app, |a, d: i32, on: bool| crate::mixer::cut_in(
        &a,
        (d as usize).saturating_sub(1),
        on
    )));
    g.on_phrase_jump(handler!(app, |a, d: i32, fwd: bool| phrase_jump(
        &a,
        (d as usize).saturating_sub(1),
        fwd
    )));
    g.on_drop_mark(handler!(app, |a, d: i32| toggle_drop(&a, (d as usize).saturating_sub(1))));
    wire_panel(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: u32, b: u32, k: PhraseKind) -> Phrase {
        Phrase::new(a, b, k)
    }

    #[test]
    fn a_jump_lands_its_target_on_the_next_downbeat() {
        // Beat 41.5 is the 2 of its bar: the next downbeat is beat 44.
        assert_eq!(next_downbeat(41.5, 2), Some(44.0));
        assert_eq!(next_downbeat(40.0, 1), Some(44.0));
        assert_eq!(next_downbeat(40.0, 0), None);
        // To phrase start 64: jump 20 beats; the playhead keeps its place
        // in the bar and reaches 64 when it would have reached 44.
        assert_eq!(landing_jump(41.5, 2, 64.0), Some(20.0));
        let after = 41.5 + 20.0;
        assert_eq!(64.0 - after, 44.0 - 41.5);
        // Back to 32 from 41.5: −12.
        assert_eq!(landing_jump(41.5, 2, 32.0), Some(-12.0));
        // Nothing to move, or off the start of the track.
        assert_eq!(landing_jump(41.5, 2, 44.0), None);
        assert_eq!(landing_jump(1.0, 2, -64.0), None);
    }

    #[test]
    fn phrase_targets_forward_and_back() {
        let ph = [
            p(0, 32, PhraseKind::Intro),
            p(32, 64, PhraseKind::Verse),
            p(64, 96, PhraseKind::Chorus),
            p(96, 128, PhraseKind::Outro),
        ];
        assert_eq!(phrase_target(&ph, 41.5, 2, true), Some(64.0));
        // In bar 3 of the verse: back to its start.
        assert_eq!(phrase_target(&ph, 41.5, 2, false), Some(32.0));
        // In its first bar: back to the phrase before.
        assert_eq!(phrase_target(&ph, 33.0, 2, false), Some(0.0));
        // The next downbeat is the next phrase: forward skips to the one after.
        assert_eq!(phrase_target(&ph, 62.0, 3, true), Some(96.0));
        // Without phrases the 16-bar marks count.
        assert_eq!(phrase_target(&[], 70.0, 3, true), Some(128.0));
        assert_eq!(phrase_target(&[], 70.0, 3, false), Some(64.0));
        assert_eq!(kind_target(&ph, 10.0, 3, PhraseKind::Chorus), Some(64.0));
        assert_eq!(kind_target(&ph, 10.0, 3, PhraseKind::Outro), Some(96.0));
        assert_eq!(kind_target(&ph, 70.0, 3, PhraseKind::Chorus), None);
    }

    #[test]
    fn the_drop_key_marks_on_a_downbeat_and_clears_near_the_mark() {
        let grid = od_core::Beatgrid::constant(120.0, 0.0, 1, 120.0);
        // 120 BPM: a bar is 2 s. At 9.3 s the nearest downbeat is 10 s.
        assert_eq!(next_drop(&grid, None, 9.3, 120.0), Some(10.0));
        // At the mark: cleared; far from it: moved.
        assert_eq!(next_drop(&grid, Some(10.0), 10.4, 120.0), None);
        assert_eq!(next_drop(&grid, Some(10.0), 30.1, 120.0), Some(30.0));
        assert_eq!(drop_beat(&grid, Some(10.0)), Some(20.0));
    }
}
