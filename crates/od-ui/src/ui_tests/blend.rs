//! For the mix: phrase-aligned blends ([56]), high tempos ([57]) and loop
//! roll and capture ([59]), driven through the real window.

use super::harness::*;
use od_controller::{ControlAction, DeckAction};
use od_core::{Beatgrid, DataSource, Phrase, PhraseKind, TrackAnalysis, TrackId};
use od_engine::DeckCommand;
use slint::ComponentHandle;
use slint::platform::PointerEventButton;

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
}

fn snapshot(ui: &Ui) -> od_engine::EngineState {
    crate::engine::snapshot(&ui.app).expect("engine runs")
}

/// Adds a `secs` long 120 BPM track, analysed: a grid from 0 s and the
/// phrases (in bars).
fn analysed(fx: &Fixture, title: &str, secs: f32, pitch: f32, phrases: &[(u32, u32, PhraseKind)]) -> TrackId {
    let path = fx.root.join(format!("music/{title}.wav"));
    write_wav(&path, secs, pitch);
    let mut lib = fx.lib();
    let id = crate::workers::import_file(&mut lib, &path).unwrap();
    let a = TrackAnalysis {
        analysis_version: od_analysis::ANALYSIS_VERSION,
        beatgrid: Beatgrid::constant(120.0, 0.0, 1, secs as f64),
        phrases: phrases
            .iter()
            .map(|&(s, e, kind)| Phrase { start_beat: s * 4, end_beat: e * 4, kind })
            .collect(),
        ..Default::default()
    };
    lib.store_analysis(id, &a, DataSource::Analysis).unwrap();
    id
}

/// Four-bar phrases (8 s at 120 BPM): intro, up, chorus, down, chorus, outro.
const PHRASES: [(u32, u32, PhraseKind); 6] = [
    (0, 4, PhraseKind::Intro),
    (4, 8, PhraseKind::Up),
    (8, 12, PhraseKind::Chorus),
    (12, 16, PhraseKind::Down),
    (16, 24, PhraseKind::Chorus),
    (24, 30, PhraseKind::Outro),
];

/// A window with two analysed 60 s tracks.
fn open() -> (Ui, [TrackId; 2]) {
    let fx = Fixture::new();
    let a = analysed(&fx, "Phrase One", 60.0, 330.0, &PHRASES);
    let b = analysed(&fx, "Phrase Two", 60.0, 440.0, &PHRASES);
    (Ui::start(fx, (1440, 900), false), [a, b])
}

fn load(ui: &Ui, deck: usize, id: TrackId) {
    crate::decks::load_track(&ui.app, deck, id, None);
    assert!(
        ui.wait_until(30.0, |u| crate::engine::snapshot(&u.app)
            .is_some_and(|s| s.decks[deck].track == Some(id))),
        "deck {} loads",
        deck + 1
    );
    ui.pump(200);
}

/// Deck 1 plays as master; deck 2 is loaded and paused.
fn two_decks(ui: &Ui, ids: [TrackId; 2]) {
    perform(ui);
    load(ui, 0, ids[0]);
    load(ui, 1, ids[1]);
    crate::engine::send(&ui.app, od_engine::Command::SetMasterDeck(Some(0)));
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Seek { seconds: 0.0 });
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Play);
    assert!(ui.wait_until(10.0, |u| snapshot(u).decks[0].playing));
}

/// [56] Bars to the drop and break on the platter; a right click on PLAY
/// arms a start on the master's next phrase (armed outline, START count,
/// the incoming outline over lane 1 with its cue mark); it starts on the
/// boundary; a second right click cancels; PHRASE sync jumps to the same
/// place in the phrase.
#[test]
fn phrase_start_overlay_and_phrase_sync() {
    let (ui, ids) = open();
    two_decks(&ui, ids);
    assert!(
        ui.wait_until(5.0, |u| u.value("deck1.phrase-ahead").contains("DROP")),
        "the platter counts bars to the drop: {:?}",
        ui.value("deck1.phrase-ahead")
    );

    // The incoming track over the playing one, its cue marked.
    ui.pump(500);
    assert!(ui.wait_until(5.0, |u| u.has("wave.incoming1") && u.value("wave.incoming1") == "2"));
    // Its cue mark comes into view as the master's next phrase nears.
    assert!(ui.wait_until(10.0, |u| u.has("wave.incoming-start1")));
    assert_eq!(ui.value("wave.incoming-start1"), "2");
    assert!(!ui.has("wave.incoming2"), "only over the playing master");

    // Arm early in a phrase: it waits, then starts on the boundary.
    assert!(ui.wait_until(5.0, |u| {
        let b = snapshot(u).decks[0].beat.rem_euclid(16.0);
        (0.3..2.0).contains(&b)
    }));
    ui.right_click_el(&ui.get("deck2.play"));
    assert!(ui.wait_until(2.0, |u| snapshot(u).decks[1].start_on_beat.is_some()), "armed");
    let target = snapshot(&ui).decks[1].start_on_beat.unwrap();
    assert_eq!(target.rem_euclid(16.0), 0.0, "a phrase boundary");
    assert!(ui.wait_until(1.0, |u| u.value("deck2.play") == "armed"));
    assert!(ui.value("deck2.phrase-ahead").starts_with("START -"));
    assert!(!snapshot(&ui).decks[1].playing, "it waits");
    let mut beat_at_start = None;
    ui.wait_until(15.0, |u| {
        let st = snapshot(u);
        if st.decks[1].playing {
            beat_at_start = Some(st.decks[0].beat);
        }
        beat_at_start.is_some()
    });
    let b = beat_at_start.expect("deck 2 started");
    assert!(
        b >= target - 0.05 && b < target + 0.5,
        "started at master beat {b}, phrase at {target}"
    );

    // Cancel: arm and press again.
    crate::engine::deck_cmd(&ui.app, 1, DeckCommand::Pause);
    assert!(ui.wait_until(2.0, |u| !snapshot(u).decks[1].playing));
    crate::controller::handle_action(
        &ui.app,
        ControlAction::Deck { deck: 2, action: DeckAction::PhraseStart },
    );
    let armed = ui.wait_until(2.0, |u| snapshot(u).decks[1].start_on_beat.is_some());
    assert!(armed, "armed again: {}", ui.app.ui.get_status_text());
    ui.right_click_el(&ui.get("deck2.play"));
    assert!(ui.wait_until(1.0, |u| snapshot(u).decks[1].start_on_beat.is_none()));
    assert!(ui.wait_until(1.0, |u| u.value("deck2.play").is_empty()));

    // PHRASE sync: deck 2 plays somewhere else in its phrase; SYNC puts it
    // as far before a boundary as the master.
    ui.app
        .state
        .borrow_mut()
        .change_settings(|s| s.deck.sync_mode = "phrase".into());
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Seek { seconds: 2.2 });
    crate::engine::deck_cmd(&ui.app, 1, DeckCommand::Seek { seconds: 21.0 });
    crate::engine::deck_cmd(&ui.app, 1, DeckCommand::Play);
    assert!(ui.wait_until(2.0, |u| snapshot(u).decks[1].playing && snapshot(u).decks[1].position > 20.0));
    ui.click("deck2.beat-sync");
    assert!(ui.wait_until(2.0, |u| snapshot(u).decks[1].sync));
    ui.wait_until(0.3, |_| false);
    let st = snapshot(&ui);
    let left = |beat: f64| 16.0 - beat.rem_euclid(16.0);
    let (m, s) = (left(st.decks[0].beat), left(st.decks[1].beat));
    let diff = (m - s).rem_euclid(16.0);
    assert!(
        diff.min(16.0 - diff) < 0.5,
        "beats to the phrase: master {m}, deck 2 {s}"
    );

    // The overlay can be switched off (Preferences › View).
    ui.app
        .state
        .borrow_mut()
        .change_settings(|s| s.view.incoming_overlay = false);
    assert!(ui.wait_until(2.0, |u| !u.has("wave.incoming1")));
}

/// [57] ½ and 2× on the BPM readout change the track's grid (one undo
/// step each); the active detection range shows in the app, 150–300
/// included; a controller can halve and double too.
#[test]
fn half_double_and_detection_range() {
    let (ui, ids) = open();
    perform(&ui);
    load(&ui, 0, ids[0]);
    let bpm = |u: &Ui| snapshot(u).decks[0].track_bpm;
    assert!((bpm(&ui) - 120.0).abs() < 0.5);
    assert!(ui.wait_until(5.0, |u| u.has("deck1.bpm-double")));
    ui.click("deck1.bpm-double");
    assert!(ui.wait_until(5.0, |u| (bpm(u) - 240.0).abs() < 1.0), "{}", bpm(&ui));
    let id = ids[0];
    let stored = ui.app.state.borrow().lib.track(id).unwrap().bpm.unwrap_or(0.0);
    assert!((stored - 240.0).abs() < 1.0, "stored with the track: {stored}");
    ui.click("deck1.bpm-half");
    assert!(ui.wait_until(5.0, |u| (bpm(u) - 120.0).abs() < 1.0));
    crate::controller::handle_action(
        &ui.app,
        ControlAction::Deck { deck: 1, action: DeckAction::BpmScale(0.5) },
    );
    assert!(ui.wait_until(5.0, |u| (bpm(u) - 60.0).abs() < 1.0));
    // One undo step back: the library has 120 again.
    ui.app.undo();
    let back = ui.app.state.borrow().lib.track(id).unwrap().bpm.unwrap_or(0.0);
    assert!((back - 120.0).abs() < 1.0, "undo: {back}");

    // The active range: 70–180 by default, then the high preset.
    ui.pump(100);
    assert_eq!(ui.app.ui.get_bpm_range_text(), "70–180");
    ui.app.ui.invoke_open_preferences();
    ui.pump(200);
    let prefs = ui
        .app
        .prefs
        .borrow()
        .window
        .as_ref()
        .map(|w| w.clone_strong())
        .expect("Preferences open");
    prefs.invoke_choose("analysis.bpm".into(), 4);
    ui.pump(100);
    assert_eq!(ui.app.ui.get_bpm_range_text(), "150–300");
    let opts = ui.app.analysis.control.lock().unwrap().options.clone();
    assert_eq!((opts.min_bpm, opts.max_bpm), (150.0, 300.0));
    let _ = prefs.hide();
}

/// [59] LOOP ROLL pads roll 1/32 … 4 beats while held, the track carries
/// on underneath (the slip line) and resumes there; the roll shows on the
/// key and the waveform; CAPTURE keeps it as a hot loop on a free pad.
#[test]
fn loop_roll_and_capture() {
    let (ui, ids) = open();
    perform(&ui);
    load(&ui, 0, ids[0]);
    ui.app.ui.invoke_deck_pad_mode_set(1, 13);
    assert!(ui.wait_until(2.0, |u| u.value("deck1.pad-mode") == "ROLL"));
    assert!(ui.has("deck1.loop.capture"));
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Play);
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].position > 1.0));

    // Hold pad C (1/8 beat).
    let pad = Rect::of(&ui.get("deck1.pad.C")).centre();
    ui.press_at(pad, PointerEventButton::Left);
    assert!(ui.wait_until(2.0, |u| snapshot(u).decks[0].loop_state.roll), "the roll runs while held");
    let st = snapshot(&ui).decks[0];
    assert!(st.loop_state.roll && st.loop_state.active);
    assert_eq!(st.loop_state.beats, Some(0.125));
    assert!(ui.wait_until(2.0, |u| u.has("wave.roll1") && u.value("wave.roll1") == "1/8"));
    assert!(ui.has("wave.slip1"), "where the track goes on underneath");
    let under = st.slip_position.expect("slip shadow");
    assert!(
        ui.wait_until(10.0, |u| snapshot(u).decks[0].slip_position.is_some_and(|x| x > under + 0.5)),
        "the track carries on underneath"
    );
    let under_later = snapshot(&ui).decks[0].slip_position.unwrap();
    ui.release_at(pad, PointerEventButton::Left);
    assert!(ui.wait_until(2.0, |u| !snapshot(u).decks[0].loop_state.roll));
    let st = snapshot(&ui).decks[0];
    assert!(!st.loop_state.roll && !st.loop_state.active);
    assert!(
        (st.position - under_later).abs() < 0.5,
        "resumes underneath: {} vs {}",
        st.position,
        under_later
    );
    assert!(ui.wait_until(1.0, |u| !u.has("wave.roll1")));

    // CAPTURE: the roll just heard becomes hot loop A.
    ui.click("deck1.loop.capture");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].hot_cues[0]
        .is_some_and(|h| h.loop_end.is_some())));
    let h = snapshot(&ui).decks[0].hot_cues[0].unwrap();
    assert!(((h.loop_end.unwrap() - h.start) - 0.0625).abs() < 0.01, "1/8 beat at 120 BPM");
    let id = ids[0];
    assert!(
        ui.wait_until(5.0, |u| u.app.state.borrow().lib.cues(id).unwrap_or_default().iter()
            .any(|c| c.kind == od_core::CueKind::Hot(0) && c.loop_end.is_some())),
        "saved with the track"
    );
    // A second capture takes the next free pad; from a controller too.
    crate::controller::handle_action(
        &ui.app,
        ControlAction::Deck { deck: 1, action: DeckAction::LoopCapture },
    );
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].hot_cues[1].is_some()));
}
