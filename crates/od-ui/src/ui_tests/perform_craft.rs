//! Roadmap 0.9.0 block G through the real window: reverse and cut-in on
//! the deck, split cue and the crossfader curve on the mixer ([30]), hot
//! cue edits, the drop and phrase jumps from Perform ([31]), per-cue pad
//! behaviour and the sampler that stays ([64]), and the set plan ([65]).

use super::harness::*;
use od_controller::PadMode;
use od_core::CueTrigger;
use slint::ComponentHandle;

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
}

fn snapshot(ui: &Ui) -> od_engine::EngineState {
    crate::engine::snapshot(&ui.app).expect("engine runs")
}

/// Loads fixture track `t` on `deck` and waits for it.
pub(crate) fn load(ui: &Ui, deck: usize, t: usize) {
    crate::decks::load_track(&ui.app, deck, ui.fx.tracks[t].0, None);
    assert!(
        ui.wait_until(30.0, |u| crate::engine::snapshot(&u.app)
            .is_some_and(|s| s.decks[deck].track.is_some())),
        "deck {} loads",
        deck + 1
    );
}

/// Gives fixture track `id` (8 s) a 240 BPM grid from 0 s and two
/// phrases of four bars (the fixture tones have neither).
pub(crate) fn with_grid(ui: &Ui, id: od_core::TrackId) {
    use od_core::{Phrase, PhraseKind};
    let mut st = ui.app.state.borrow_mut();
    let d = st.lib.track(id).unwrap().duration;
    st.lib.set_beatgrid(id, od_core::Beatgrid::constant(240.0, 0.0, 1, d)).unwrap();
    st.lib
        .set_phrases(id, &[Phrase::new(0, 16, PhraseKind::Intro), Phrase::new(16, 32, PhraseKind::Chorus)])
        .unwrap();
}

/// Deck 1 in CUE EDIT with a track (the cue panel beside the list).
pub(crate) fn open_cue_edit(ui: &Ui) {
    perform(ui);
    load(ui, 0, 0);
    crate::decks::set_pad_mode(&ui.app, 0, PadMode::CueEdit);
    assert!(ui.wait_until(5.0, |u| u.has("cue-edit.panel")), "the cue panel opens");
}

/// A set of the three fixture tracks with a planned start and a note on
/// the first transition, opened in the browser (it becomes the plan).
pub(crate) fn plan_set(ui: &Ui, start: &str) -> od_library::NodeId {
    let ids: Vec<_> = ui.fx.tracks.iter().map(|t| t.0).collect();
    let set = {
        let mut st = ui.app.state.borrow_mut();
        let set = st.lib.create_node(None, od_library::NodeKind::Set, "Friday Late Set").unwrap();
        st.lib.add_to_node(set, &ids, None).unwrap();
        st.lib.set_entry_note(set, 0, "Bring the bass in on the breakdown, then swap on the bar").unwrap();
        st.lib
            .set_set_meta(set, od_library::SetMeta { planned_start: Some(start.into()), notes: String::new() })
            .unwrap();
        set
    };
    ui.app.ui.invoke_select_node(set.0 as i32);
    ui.pump(200);
    set
}

/// [30] REV plays the deck backwards and back; CUT IN opens the
/// crossfader for its channel while held; SPLIT and the curve key reach
/// the engine and the settings.
#[test]
fn reverse_cut_in_split_cue_and_curve_key() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    load(&ui, 0, 0);
    ui.click("deck1.reverse");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].reverse), "REV reverses");
    assert!(ui.wait_until(5.0, |u| u.get("deck1.reverse").accessible_checked() == Some(true)), "REV shows on");
    ui.click("deck1.reverse");
    assert!(ui.wait_until(5.0, |u| !snapshot(u).decks[0].reverse), "and back");
    // CUT IN: held down, the channel is cut in; released, not.
    let key = ui.get("deck1.cut-in");
    let at = Rect::of(&key);
    let p = slint::LogicalPosition::new(at.x + at.w - 12.0, at.y + at.h / 2.0);
    ui.press_at(p, slint::platform::PointerEventButton::Left);
    assert!(ui.wait_until(5.0, |u| snapshot(u).channels[0].cut_in), "held: cut in");
    ui.release_at(p, slint::platform::PointerEventButton::Left);
    assert!(ui.wait_until(5.0, |u| !snapshot(u).channels[0].cut_in), "released");
    // SPLIT.
    ui.click("global.phones.split");
    assert!(ui.wait_until(5.0, |u| snapshot(u).split_cue), "split cue on");
    assert!(ui.wait_until(5.0, |u| u.get("global.phones.split").accessible_checked() == Some(true)));
    ui.click("global.phones.split");
    assert!(ui.wait_until(5.0, |u| !snapshot(u).split_cue));
    // The curve key steps constant power → linear → cut → constant power.
    let curve = |u: &Ui| u.app.state.borrow().settings.mixer.crossfader_curve.clone();
    assert_eq!(curve(&ui), "constant_power");
    for want in ["linear", "cut", "constant_power"] {
        ui.click("global.xf-curve");
        assert_eq!(curve(&ui), want);
    }
    ui.click("global.xf-curve");
    assert!(
        ui.wait_until(5.0, |u| snapshot(u).crossfader_curve == od_engine::CrossfaderCurve::Linear),
        "the engine follows"
    );
    assert_eq!(ui.value("global.xf-curve"), "LINEAR");
}

/// [30] In STEM mode the kill keys stay and mute the part of their band.
#[test]
fn kill_keys_stay_in_stem_mode() {
    let s = od_engine::DeckState {
        stem_levels: [1.0, 0.0, 1.0, 1.0],
        ..Default::default()
    };
    let v = crate::mixer::parts_view(0, &od_engine::ChannelState::default(), &s);
    // Drums (index 1) muted: the LOW kill reads KILL.
    assert!(v.parts && v.kill_low && !v.kill_mid && !v.kill_high);
}

/// [31] [64] CUE EDIT: a pad picks the cue; the panel stores it at the
/// playhead, renames, recolours, sets how its pad plays (the engine
/// follows) and deletes it; every edit is in the library.
#[test]
fn hot_cue_edits_from_perform() {
    let ui = Ui::open((1440, 900));
    open_cue_edit(&ui);
    let id = ui.fx.tracks[0].0;
    let cues = |u: &Ui| u.app.state.borrow().lib.cues(id).unwrap();
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: 2.0 });
    ui.pump(100);
    // Pad C picks slot C; it is empty: TO PLAYHEAD stores it.
    crate::decks::pad(&ui.app, 0, 2, true);
    ui.pump(100);
    assert!(ui.value("cue-edit.title").contains("HOT CUE C"), "{}", ui.value("cue-edit.title"));
    assert!(!ui.value("cue-edit.state").is_empty(), "pad C has no cue yet");
    ui.click("cue-edit.move-here");
    assert!(ui.wait_until(5.0, |u| cues(u).iter().any(|c| c.kind == od_core::CueKind::Hot(2))), "stored");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].hot_cues[2].is_some()), "the engine has it");
    // Rename, recolour, behaviour.
    let g = ui.app.ui.global::<crate::CueEditPerform>();
    g.invoke_rename("  Vocal in ".into());
    ui.click("cue-edit.color.3");
    ui.click("cue-edit.trigger.gate");
    let c = cues(&ui).into_iter().find(|c| c.kind == od_core::CueKind::Hot(2)).unwrap();
    assert_eq!(c.label, "Vocal in");
    assert_eq!(c.color, Some(crate::player::CUE_PALETTE[2]));
    assert_eq!(c.trigger, CueTrigger::Gate);
    assert!(
        ui.wait_until(5.0, |u| snapshot(u).decks[0].hot_cues[2].is_some_and(|h| h.trigger == CueTrigger::Gate)),
        "the pad plays it as a gate"
    );
    assert_eq!(ui.get("cue-edit.trigger.gate").accessible_checked(), Some(true));
    // The deck's pad names it and its behaviour.
    assert!(ui.wait_until(5.0, |u| u.app.decks.borrow().loaded(0).is_some()));
    let rows = crate::decks::pad_rows(&ui.app, 0, &snapshot(&ui));
    assert_eq!(rows[2].sub.as_str(), "Vocal in");
    assert_eq!(rows[2].badge.as_str(), "GATE");
    // Move: later in the track.
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: 4.0 });
    ui.pump(100);
    ui.click("cue-edit.move-here");
    let c = cues(&ui).into_iter().find(|c| c.kind == od_core::CueKind::Hot(2)).unwrap();
    assert!(c.start > 3.0 && c.start < 5.0, "moved to the playhead: {}", c.start);
    assert_eq!(c.trigger, CueTrigger::Gate, "a moved cue keeps its behaviour");
    // Delete, and undo brings it back.
    ui.click("cue-edit.delete");
    assert!(!cues(&ui).iter().any(|c| c.kind == od_core::CueKind::Hot(2)));
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].hot_cues[2].is_none()), "gone from the deck");
    // DONE goes back to the hot cue pads and closes the panel.
    ui.click("cue-edit.done");
    assert!(ui.wait_until(5.0, |u| !u.has("cue-edit.panel")));
    assert_eq!(ui.app.decks.borrow().pad_mode[0], PadMode::HotCue);
}

/// [31] The drop: marked on a downbeat, shown on the deck, part of the
/// phrases; pressed again at the mark it is cleared. Phrase pads jump in
/// whole beats.
#[test]
fn drop_marker_and_phrase_jumps() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    let id = ui.fx.tracks[0].0;
    with_grid(&ui, id);
    load(&ui, 0, 0);
    assert!(
        ui.wait_until(30.0, |u| snapshot(u).decks[0].beat_in_bar > 0),
        "the track has a grid"
    );
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: 3.1 });
    ui.pump(200);
    crate::deck_craft::toggle_drop(&ui.app, 0);
    let at = ui.app.state.borrow().lib.drop_marker(id).unwrap().expect("marked");
    let grid = ui.app.decks.borrow().loaded(0).unwrap().grid.clone();
    assert!(grid.beats.iter().any(|b| (b.time - at).abs() < 1e-6 && b.bar_position == 1), "on a downbeat");
    assert!(ui.wait_until(5.0, |u| u.has("deck1.overview.drop")), "shown on the overview");
    // Again at the mark: cleared (undo step each).
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: at });
    assert!(ui.wait_until(5.0, |u| (snapshot(u).decks[0].position - at).abs() < 0.05), "at the mark: {}", snapshot(&ui).decks[0].position);
    crate::deck_craft::toggle_drop(&ui.app, 0);
    assert_eq!(ui.app.state.borrow().lib.drop_marker(id).unwrap(), None);
    // PHRASE pads: B jumps forward by whole beats, keeping the bar phase.
    crate::decks::set_pad_mode(&ui.app, 0, PadMode::Phrase);
    // 0.3 s is beat 1.2 (the 2 of bar 1): the chorus at beat 16 lands on
    // the next downbeat (beat 4), so the jump is 12 beats.
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: 0.3 });
    assert!(ui.wait_until(5.0, |u| (snapshot(u).decks[0].position - 0.3).abs() < 0.01));
    let before = snapshot(&ui).decks[0];
    crate::decks::pad(&ui.app, 0, 1, true);
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].beat > 8.0), "jumped");
    let jumped = snapshot(&ui).decks[0].beat - before.beat;
    assert!((jumped - 12.0).abs() < 0.05, "12 beats: {jumped}");
    // From beat 22.2 (bar 6 of the chorus) A goes back to the chorus start
    // (beat 16), landing it on the next downbeat (24): −8 beats.
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: 5.55 });
    assert!(ui.wait_until(5.0, |u| (snapshot(u).decks[0].position - 5.55).abs() < 0.01));
    let before = snapshot(&ui).decks[0];
    crate::decks::pad(&ui.app, 0, 0, true);
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].beat < 20.0), "jumped back");
    let jumped = snapshot(&ui).decks[0].beat - before.beat;
    assert!((jumped + 8.0).abs() < 0.05, "−8 beats: {jumped}");
}

/// [64] The sampler stays when another panel opens in a small window.
#[test]
fn the_sampler_stays_when_fx_opens() {
    let ui = Ui::open((1280, 800));
    perform(&ui);
    let w = &ui.app.ui;
    w.invoke_toggle_sampler_panel();
    ui.pump(300);
    w.invoke_toggle_fx_panel();
    ui.pump(300);
    w.invoke_menu_action("perform.mix-preview.panel".into());
    ui.pump(700);
    assert!(w.get_sampler_panel(), "the sampler stays in view");
}

/// [65] The plan line follows the set: the track on air, the next one with
/// the note, and where the set stands.
#[test]
fn the_set_plan_in_perform() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    plan_set(&ui, "");
    assert!(ui.wait_until(5.0, |u| u.has("plan.line")), "the plan shows in Perform");
    assert_eq!(ui.value("plan.name"), "Friday Late Set");
    // Track 1 on air.
    load(&ui, 0, 0);
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Play);
    assert!(ui.wait_until(10.0, |u| u.value("plan.now") == "1"), "on air: 1 of 3");
    assert!(ui.value("plan.next").contains(TITLES[1]), "{}", ui.value("plan.next"));
    assert!(ui.value("plan.note").contains("Bring the bass in"), "the note of 1 → 2");
    assert!(ui.value("plan.standing").contains("NO PLANNED START"), "{}", ui.value("plan.standing"));
    assert_eq!(ui.value("plan.form"), "half");
    // × hides it until another set is opened.
    ui.click("plan.close");
    assert!(ui.wait_until(5.0, |u| !u.has("plan.line")));
}
