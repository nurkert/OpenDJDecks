//! EDIT IN PREPARE through the real window: from a playing Perform deck to
//! its track in the Prepare player and back, the deck menu's editors and
//! quick fixes, and every fix reaching the loaded deck at once while the
//! audio plays on.

use super::harness::*;
use od_core::{
    AnalysisConfidence, Beatgrid, Cue, DataSource, MusicalKey, Phrase, PhraseKind, TrackAnalysis,
    TrackId,
};
use od_engine::DeckCommand;
use slint::ComponentHandle;
use slint::platform::{Key, PointerEventButton};
use std::sync::Arc;

fn key(s: &str) -> MusicalKey {
    MusicalKey::parse(s).unwrap()
}

/// A 60 s track at 120 BPM in 8A (the analysis unsure, 8B likely), with
/// three phrases and a hot cue, analysed.
fn track(fx: &Fixture) -> TrackId {
    let path = fx.root.join("music/Edit Me.wav");
    write_wav(&path, 60.0, 330.0);
    let mut lib = fx.lib();
    let id = crate::workers::import_file(&mut lib, &path).unwrap();
    let mut tags = lib.track(id).unwrap().tags;
    tags.title = "Edit Me".into();
    lib.set_tags(id, tags).unwrap();
    let a = TrackAnalysis {
        analysis_version: od_analysis::ANALYSIS_VERSION,
        beatgrid: Beatgrid::constant(120.0, 0.0, 1, 60.0),
        key: Some(key("8A")),
        confidence: Some(AnalysisConfidence { key: 0.3, key_alt: Some(key("8B")), grid: 0.9, bpm_alt: None }),
        ..Default::default()
    };
    lib.store_analysis(id, &a, DataSource::Analysis).unwrap();
    lib.set_phrases(
        id,
        &[
            Phrase::new(0, 32, PhraseKind::Intro),
            Phrase::new(32, 64, PhraseKind::Verse),
            Phrase::new(64, 120, PhraseKind::Chorus),
        ],
    )
    .unwrap();
    lib.set_cues(id, vec![Cue::hot(0, 1.0)]).unwrap();
    id
}

fn snapshot(ui: &Ui) -> od_engine::EngineState {
    crate::engine::snapshot(&ui.app).expect("engine runs")
}

/// Perform with the track playing on deck 1 in a 4-beat loop at 14–16 s,
/// beats 28–32 (it plays on for as long as the test needs).
fn playing(ui: &Ui, id: TrackId) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
    crate::decks::load_track(&ui.app, 0, id, None);
    assert!(
        ui.wait_until(60.0, |u| crate::engine::snapshot(&u.app).is_some_and(|s| s.decks[0].track == Some(id))),
        "deck 1 loads"
    );
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Seek { seconds: 14.0 });
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::AutoLoop { beats: 4.0 });
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Play);
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].playing), "deck 1 plays");
    ui.pump(200);
}

/// The deck plays on: still playing, and its position moves.
fn plays_on(ui: &Ui) -> bool {
    let a = snapshot(ui).decks[0].position;
    ui.wait_until(3.0, |u| {
        let s = snapshot(u).decks[0];
        s.playing && (s.position - a).abs() > 0.01
    })
}

fn engine_track(ui: &Ui) -> Arc<od_engine::TrackData> {
    ui.app.decks.borrow().engine_track(0).expect("deck 1 has its track data")
}

/// The EDIT key in the deck header: the deck's track in the Prepare player
/// at the deck's position with the PHRASE tab open, the audio playing on
/// without a reload; Esc returns to Perform as it was (the GRID panel of
/// deck 2 still open, the Prepare layout as before).
#[test]
fn edit_key_opens_the_track_in_prepare_and_esc_returns() {
    let fx = Fixture::new();
    let id = track(&fx);
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_menu_action("view.layout.browser".into());
    playing(&ui, id);
    ui.click("deck2.grid-panel");
    assert!(ui.wait_until(2.0, |u| u.has("deck2.grid.click")), "deck 2's GRID panel is open");
    let audio = engine_track(&ui);
    assert!(ui.has("deck1.edit"));
    ui.click("deck1.edit");
    assert_eq!(ui.app.ui.get_mode(), "prepare");
    assert_eq!(ui.app.state.borrow().player.current, Some(id), "the deck's track");
    let (cursor, pos) = (ui.app.state.borrow().player.cursor, snapshot(&ui).decks[0].position);
    assert!((14.0..=16.5).contains(&cursor) && (cursor - pos).abs() < 2.0, "at the deck's place: {cursor} / {pos}");
    assert_eq!(ui.app.ui.get_prepare_layout(), "player", "the player shows");
    assert_eq!(ui.app.ui.global::<crate::PlayerTabs>().get_tool(), "phrase", "phrases by default");
    assert!(ui.wait_until(2.0, |u| u.has("player1.tab.phrase") && u.has("topbar.back-to-perform")));
    assert!(plays_on(&ui), "the deck plays on in Prepare");
    assert!(Arc::ptr_eq(&audio, &engine_track(&ui)), "no reload");
    assert!(!ui.app.state.borrow().player.previewing, "nothing else sounds");

    ui.key(Key::Escape);
    assert_eq!(ui.app.ui.get_mode(), "perform", "Esc returns");
    assert!(!crate::edit_in_prepare::from_perform());
    assert_eq!(ui.app.ui.get_prepare_layout(), "browser", "Prepare as it was");
    assert!(ui.wait_until(2.0, |u| u.has("deck2.grid.click")), "deck 2's GRID panel is still open");
    assert!(plays_on(&ui), "and deck 1 still plays");
    assert!(Arc::ptr_eq(&audio, &engine_track(&ui)), "the same audio throughout");
    // Esc in Prepare without a trip from Perform stays Prepare's.
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.key(Key::Escape);
    assert_eq!(ui.app.ui.get_mode(), "prepare");
}

/// BACK TO PERFORM, the keyboard shortcut (again: back) and the controller
/// action, and the mode tab: each ends the trip.
#[test]
fn back_to_perform_by_key_shortcut_controller_and_tab() {
    let fx = Fixture::new();
    let id = track(&fx);
    let ui = Ui::start(fx, (1440, 900), false);
    playing(&ui, id);
    ui.click("deck1.edit");
    assert!(ui.wait_until(2.0, |u| u.has("topbar.back-to-perform")));
    ui.click("topbar.back-to-perform");
    assert_eq!(ui.app.ui.get_mode(), "perform");
    assert!(!ui.app.ui.global::<crate::EditInPrepare>().get_from_perform());

    crate::commands::run(&ui.app, "deck1.edit");
    assert_eq!(ui.app.ui.get_mode(), "prepare", "the shortcut opens");
    crate::commands::run(&ui.app, "deck1.edit");
    assert_eq!(ui.app.ui.get_mode(), "perform", "and again returns");

    crate::controller::handle_action(
        &ui.app,
        od_controller::ControlAction::Deck { deck: 1, action: od_controller::DeckAction::EditInPrepare },
    );
    assert_eq!(ui.app.ui.get_mode(), "prepare", "the controller action opens");
    ui.click("topbar.mode.perform");
    assert_eq!(ui.app.ui.get_mode(), "perform");
    assert!(!crate::edit_in_prepare::from_perform(), "the mode tab ends the trip");
    assert!(plays_on(&ui));
}

/// The deck's right-click menu opens Prepare with the editor asked for:
/// grid (EDIT GRID), cues, key and gain (INFO), track info (info panel),
/// stems.
#[test]
fn deck_menu_opens_each_editor() {
    let fx = Fixture::new();
    let id = track(&fx);
    let ui = Ui::start(fx, (1440, 900), false);
    playing(&ui, id);
    let tabs = |u: &Ui| {
        let t = u.app.ui.global::<crate::PlayerTabs>();
        (t.get_tool().to_string(), t.get_list().to_string(), t.get_focus().to_string())
    };
    let menu = |u: &Ui, title: &str| {
        u.app.ui.invoke_set_mode("perform".into());
        u.pump(200);
        let lane = Rect::of(&u.get("wave.lane1"));
        u.click_at(slint::LogicalPosition::new(lane.x + lane.w * 0.3, lane.y + lane.h / 2.0), PointerEventButton::Right);
        u.pump(100);
        assert!(u.click_menu_item(title), "{title}");
        assert_eq!(u.app.ui.get_mode(), "prepare", "{title}");
    };
    menu(&ui, "Edit beatgrid…");
    assert!(ui.app.ui.global::<crate::GridEdit>().get_active(), "EDIT GRID runs");
    menu(&ui, "Edit cues…");
    assert!(!ui.app.ui.global::<crate::GridEdit>().get_active(), "back in Perform kept the grid (DONE)");
    assert_eq!(tabs(&ui).0, "cue");
    assert_eq!(tabs(&ui).1, "all");
    menu(&ui, "Edit key…");
    assert_eq!((tabs(&ui).1.as_str(), tabs(&ui).2.as_str()), ("info", "key"));
    menu(&ui, "Edit gain…");
    assert_eq!(tabs(&ui).2, "gain");
    assert!(ui.wait_until(5.0, |u| {
        let _ = u.find("player1.bpm-range").map(|e| Rect::of(&e));
        u.has("player1.gain-up")
    }), "GAIN with its keys in INFO");
    menu(&ui, "Edit stems…");
    assert_eq!(tabs(&ui).0, "stems");
    menu(&ui, "Edit track info…");
    assert!(ui.app.ui.get_info_open(), "the info panel");
    ui.key(Key::Escape);
    assert!(!ui.app.ui.get_info_open(), "closed again on the way back");
    menu(&ui, "Edit phrases…");
    assert_eq!(tabs(&ui).0, "phrase");
    assert!(plays_on(&ui));
}

/// Quick fixes in place, from the menu of the waveform, the phrase strip
/// and the key: the downbeat one beat, the phrase boundary under the
/// playhead one bar, the likely key; each is one undo step and the deck
/// takes it at once, Perform stays.
#[test]
fn quick_fixes_stay_in_perform_and_undo_one_by_one() {
    let fx = Fixture::new();
    let id = track(&fx);
    let ui = Ui::start(fx, (1440, 900), false);
    playing(&ui, id);
    let lib = |u: &Ui| u.fx.lib();
    let first_downbeat = |u: &Ui| {
        let g = lib(u).beatgrid(id).unwrap().unwrap();
        g.beats.iter().find(|b| b.bar_position == 1).unwrap().time
    };
    let before = first_downbeat(&ui);
    let right_click = |u: &Ui, label: &str| {
        let r = Rect::of(&u.get(label));
        u.click_at(slint::LogicalPosition::new(r.x + r.w * 0.3, r.y + r.h / 2.0), PointerEventButton::Right);
        u.pump(100);
    };
    right_click(&ui, "wave.lane1");
    assert!(ui.click_menu_item("Downbeat one beat later"));
    assert_eq!(ui.app.ui.get_mode(), "perform", "in place");
    assert!(ui.wait_until(2.0, |u| (first_downbeat(u) - before).abs() > 0.4), "the 1 moved a beat");
    let deck_grid = ui.app.decks.borrow().loaded(0).unwrap().grid.clone();
    assert_eq!(Some(deck_grid), lib(&ui).beatgrid(id).unwrap(), "the deck has the new grid");

    // The playhead loops at beats 28–32: the boundary at beat 32 is the
    // nearest.
    let strip = ui.get("deck1.overview");
    let r = Rect::of(&strip);
    ui.click_at(slint::LogicalPosition::new(r.x + r.w * 0.5, r.bottom() + 5.0), PointerEventButton::Right);
    ui.pump(100);
    assert!(ui.click_menu_item("Phrase boundary one bar later"), "from the phrase strip");
    let bounds = |u: &Ui| lib(u).phrases(id).unwrap().iter().map(|p| p.start_beat).collect::<Vec<_>>();
    assert!(ui.wait_until(2.0, |u| bounds(u) == vec![0, 36, 64]), "{:?}", bounds(&ui));
    assert!(ui.wait_until(2.0, |u| u.app.decks.borrow().loaded(0).unwrap().phrases.iter().any(|p| p.start_beat == 36)), "the deck's phrases");

    let k = Rect::of(&ui.get("deck1.key"));
    ui.click_at(k.centre(), PointerEventButton::Right);
    ui.pump(100);
    assert!(ui.click_menu_item("Use the likely key 8B"), "from the key");
    assert!(ui.wait_until(2.0, |u| lib(u).track(id).unwrap().key == Some(key("8B"))));
    assert!(ui.wait_until(2.0, |u| snapshot(u).decks[0].track_key == Some(key("8B"))), "the engine's key");
    assert!(ui.wait_until(2.0, |u| u.value("deck1.key") == "8B"), "the deck shows it");
    assert_eq!(ui.app.ui.get_mode(), "perform");

    // One undo step each, newest first.
    ui.app.undo();
    assert_eq!(lib(&ui).track(id).unwrap().key, Some(key("8A")));
    assert!(ui.wait_until(2.0, |u| u.value("deck1.key") == "8A"), "the deck follows the undo");
    ui.app.undo();
    assert_eq!(bounds(&ui), vec![0, 32, 64]);
    ui.app.undo();
    assert!((first_downbeat(&ui) - before).abs() < 0.01, "the grid as it was");
    assert!(plays_on(&ui));
}

/// Every fix made in Prepare reaches the loaded deck at once, with the
/// audio playing on: phrases (countdowns and blends read them), grid,
/// key, cues (the engine's hot cues), gain and the title.
#[test]
fn fixes_in_prepare_reach_the_loaded_deck() {
    let fx = Fixture::new();
    let id = track(&fx);
    let ui = Ui::start(fx, (1440, 900), false);
    playing(&ui, id);
    let audio = engine_track(&ui);
    ui.click("deck1.edit");
    assert_eq!(ui.app.ui.get_mode(), "prepare");
    let deck = |u: &Ui| u.app.decks.borrow().loaded(0).map(|m| (m.phrases.clone(), m.grid.clone(), m.title.clone(), m.gain_adjust)).unwrap();

    // Phrases: the PHRASE tab's cut at the cursor.
    crate::player::set_cursor(&ui.app, 12.0);
    crate::phrase_edit::edit(&ui.app, crate::phrase_edit::Op::Kind(PhraseKind::Bridge));
    assert!(ui.wait_until(2.0, |u| deck(u).0.iter().any(|p| p.kind == PhraseKind::Bridge)), "phrases");
    let next = crate::phrase_mix::next_boundary(&deck(&ui).0, 9.0, 0.0);
    assert_eq!(next, 32.0, "phrase countdowns read the new phrases");

    // Grid.
    let mut st_grid = ui.fx.lib().beatgrid(id).unwrap().unwrap();
    st_grid.beats.iter_mut().for_each(|b| b.time += 0.01);
    ui.app.state.borrow_mut().lib.set_beatgrid(id, st_grid.clone()).unwrap();
    ui.app.after_edit("");
    assert!(ui.wait_until(2.0, |u| deck(u).1.beats[0].time == st_grid.beats[0].time), "grid");
    assert_eq!(engine_track(&ui).beatgrid.beats[0].time, st_grid.beats[0].time, "the engine's grid");

    // Key.
    ui.app.state.borrow_mut().lib.set_key(id, Some(key("3A"))).unwrap();
    ui.app.after_edit("");
    assert!(ui.wait_until(2.0, |u| snapshot(u).decks[0].track_key == Some(key("3A"))), "key");
    let shown = key("3A").format(ui.app.state.borrow().settings.key_notation());
    assert_eq!(ui.app.decks.borrow().key_and_cues(0).unwrap().0, shown, "the deck's key");

    // Cues.
    ui.app.state.borrow_mut().lib.set_cues(id, vec![Cue::hot(0, 1.0), Cue::hot(2, 3.0)]).unwrap();
    ui.app.after_edit("");
    assert!(ui.wait_until(2.0, |u| u.app.decks.borrow().key_and_cues(0).unwrap().1.len() == 2), "cues");
    let t = engine_track(&ui);
    assert!(t.hot_cues[0][2].is_some_and(|c| (c.start - 3.0).abs() < 1e-9), "the engine's hot cue C");

    // Gain: GAIN + in INFO.
    crate::player::act(&ui.app, "gain-up");
    assert!(ui.wait_until(2.0, |u| (deck(u).3 - 0.5).abs() < 1e-6), "gain");
    assert_eq!(ui.app.ui.get_track_gain(), "+0.5 dB");

    // Info: the title.
    let mut tags = ui.fx.lib().track(id).unwrap().tags;
    tags.title = "Edited Title".into();
    ui.app.state.borrow_mut().lib.set_tags(id, tags).unwrap();
    ui.app.after_edit("");
    assert!(ui.wait_until(2.0, |u| deck(u).2 == "Edited Title"), "title");

    assert!(plays_on(&ui), "the deck played on through every fix");
    assert!(Arc::ptr_eq(&audio.samples, &engine_track(&ui).samples), "the same audio, never decoded again");
    ui.click("topbar.back-to-perform");
    assert!(ui.wait_until(2.0, |u| u.value("deck1.title") == "Edited Title"), "Perform shows the fixes");
}
