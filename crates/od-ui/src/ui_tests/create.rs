//! Roadmap [70]: the CREATE tab, its three views, the result bar and the
//! deck strip that keeps the decks in view while they play.

use super::harness::*;
use slint::{ComponentHandle, Model};

fn snapshot(ui: &Ui) -> od_engine::EngineState {
    crate::engine::snapshot(&ui.app).expect("engine runs")
}

/// The top bar's CREATE tab (and its shortcut) opens the view; MELODY,
/// DRUMS and THEORY each show their zones, the result bar and the strip;
/// no browser takes room under it.
#[test]
fn create_tab_shows_three_views_with_result_and_deck_strip() {
    let ui = Ui::open((1440, 900));
    assert!(ui.has("topbar.mode.create"), "the fourth tab");
    ui.click("topbar.mode.create");
    ui.pump(300);
    assert_eq!(ui.app.ui.get_mode(), "create");
    assert!(!ui.has("browser.rows"), "no browser under CREATE");
    let zones = [
        ("melody", ["create.melody.source", "create.melody.notes", "create.melody.fits"]),
        ("drums", ["create.drums.kit", "create.drums.pads", "create.drums.steps"]),
        ("theory", ["create.theory.keys", "create.theory.check", "create.theory.why"]),
    ];
    for (view, ids) in zones {
        ui.click(&format!("create.view.{view}"));
        ui.pump(200);
        assert_eq!(ui.app.ui.global::<crate::Create>().get_view(), view);
        for id in ids {
            assert!(ui.has(id), "{view}: {id}");
        }
        assert!(!ui.value("create.result.summary").is_empty(), "{view}: the result says what comes there");
        assert!(ui.has("create.strip.deck1") && ui.has("create.strip.deck2"), "{view}: the deck strip");
    }
    // The later controls are there by name, but disabled.
    ui.click("create.view.melody");
    ui.pump(200);
    for id in ["create.source.hear", "create.result.send", "create.result.render"] {
        assert_eq!(ui.get(id).accessible_enabled(), Some(false), "{id} waits for its step");
    }
    // The shortcut and the command reach it from another mode.
    ui.app.ui.invoke_set_mode("prepare".into());
    crate::commands::run(&ui.app, "mode.create");
    assert_eq!(ui.app.ui.get_mode(), "create");
}

/// The decks keep playing while CREATE is open: nothing pauses or
/// unloads, and the strip shows deck 1 playing with its tempo.
#[test]
fn decks_keep_playing_in_create_and_the_strip_follows_them() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(200);
    let id = ui.fx.tracks[0].0;
    super::perform_craft::with_grid(&ui, id);
    crate::decks::load_track(&ui.app, 0, id, None);
    assert!(ui.wait_until(30.0, |u| snapshot(u).decks[0].track.is_some()), "deck 1 loads");
    ui.click("deck1.play");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].playing), "deck 1 plays");
    let before = snapshot(&ui).decks[0].position;
    ui.click("topbar.mode.create");
    assert!(
        ui.wait_until(5.0, |u| u.value("create.strip.deck1").starts_with("PLAYING")),
        "the strip shows deck 1 playing: {:?}",
        ui.value("create.strip.deck1")
    );
    assert!(ui.value("create.strip.deck1").contains("BPM"), "with its tempo: {:?}", ui.value("create.strip.deck1"));
    assert!(ui.value("create.strip.deck2").starts_with("EMPTY"), "deck 2 is empty");
    assert!(ui.has("create.strip.deck1.bpm") && ui.has("create.strip.deck1.phase") && ui.has("create.strip.deck1.level"));
    ui.pump(500);
    let s = snapshot(&ui);
    assert!(s.decks[0].playing && s.decks[0].track == Some(id), "still playing, still loaded");
    assert!(s.decks[0].position > before, "and moving on");
    // Every view, and back to Perform: still playing.
    for v in ["drums", "theory", "melody"] {
        ui.click(&format!("create.view.{v}"));
        ui.pump(100);
    }
    ui.click("topbar.mode.perform");
    ui.pump(200);
    assert!(snapshot(&ui).decks[0].playing, "playing after CREATE");
}

/// Below about 1060 px the strip keeps the play states only.
#[test]
fn narrow_strip_keeps_the_play_states() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("create".into());
    ui.pump(300);
    assert!(ui.has("create.strip.deck1.bpm"), "tempo in a wide window");
    assert!(ui.has("create.melody.fits"), "FITS in a wide window");
    ui.resize((1000, 700));
    ui.pump(300);
    assert!(ui.has("create.strip.deck1") && ui.has("create.strip.deck2"), "the play states stay");
    assert!(!ui.has("create.strip.deck1.bpm") && !ui.has("create.strip.deck1.phase"), "tempo and phase fold away");
    assert!(!ui.has("create.melody.fits"), "FITS folds away (THEORY has it)");
}

/// The mode and the view of CREATE come back after a restart.
#[test]
fn create_mode_and_view_are_remembered() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("create".into());
    crate::create::set_view(&ui.app, "drums");
    crate::shell::remember_view(&ui.app);
    ui.app.ui.invoke_set_mode("prepare".into());
    crate::create::set_view(&ui.app, "melody");
    crate::create::set_view(&ui.app, "no-such-view");
    assert_eq!(ui.app.ui.global::<crate::Create>().get_view(), "melody", "unknown views are ignored");
    crate::shell::restore_view(&ui.app);
    assert_eq!(ui.app.ui.get_mode(), "create");
    assert_eq!(ui.app.ui.global::<crate::Create>().get_view(), "drums");
}

/// The fourth tab costs no row: at the laptop size the four modes and the
/// live values still share one row of the top bar in Perform.
#[test]
fn four_modes_and_the_live_values_share_one_row_at_laptop_size() {
    let ui = Ui::open((1280, 800));
    crate::engine::ensure_engine(&ui.app);
    ui.app.ui.invoke_set_mode("perform".into());
    ui.pump(500);
    let tab = Rect::of(&ui.get("topbar.mode.create"));
    for id in ["topbar.status", "topbar.settings", "topbar.clock"] {
        assert!((Rect::of(&ui.get(id)).y - tab.y).abs() < 1.0, "{id} on the row of the modes");
    }
}

// ── [71] The picked note ────────────────────────────────────────────────

fn note_state(ui: &Ui) -> String {
    ui.value("create.source.badge")
}

/// Deck 1 holding the fixture's first track (a 220 Hz tone under a click
/// every half second), paused at `at` seconds, and CREATE open.
fn deck1_at(ui: &Ui, at: f64) -> od_core::TrackId {
    crate::engine::ensure_engine(&ui.app);
    let id = ui.fx.tracks[0].0;
    crate::decks::load_track(&ui.app, 0, id, None);
    assert!(ui.wait_until(30.0, |u| snapshot(u).decks[0].track == Some(id)), "deck 1 loads");
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: at });
    assert!(ui.wait_until(5.0, |u| (snapshot(u).decks[0].position - at).abs() < 0.01), "deck 1 at {at} s");
    ui.click("topbar.mode.create");
    ui.click("create.view.melody");
    assert!(ui.wait_until(5.0, |u| u.get("create.source.deck1").accessible_enabled() == Some(true)), "DECK 1 can be picked");
    id
}

/// Picks from deck 1 where it stands; the waveform shows IN and OUT, the
/// pitch reads as a note and cents with DETECTED, OUT dragged in measures
/// again, HEAR plays the note beside the paused deck, a step and a typed
/// name correct it (SET BY YOU), and the note is back after a restart.
#[test]
fn a_note_picked_from_a_deck_shows_its_pitch_and_takes_a_correction() {
    let ui = Ui::open((1440, 900));
    let id = deck1_at(&ui, 1.04);
    assert_eq!(ui.get("create.source.hear").accessible_enabled(), Some(false), "nothing to hear yet");
    assert!(!ui.has("create.source.pitch"), "no pitch before a pick");
    ui.click("create.source.deck1");
    assert!(ui.wait_until(5.0, |u| note_state(u) != "MEASURING" && u.has("create.source.pitch")), "measured");
    let n = crate::create::note(&ui.app).expect("a note");
    assert_eq!((n.track, n.source.as_str()), (Some(id), "deck1"));
    assert!((n.start - 1.04).abs() < 0.02, "IN at the playhead: {}", n.start);
    for h in ["create.source.wave", "create.source.in", "create.source.out", "create.source.track"] {
        assert!(ui.has(h), "{h}");
    }
    assert!(ui.value("create.source.track").starts_with(&ui.fx.tracks[0].1), "the track: {:?}", ui.value("create.source.track"));
    // OUT dragged in before the next click (at 1.5 s): only the tone is left.
    let wave = Rect::of(&ui.get("create.source.wave"));
    let out = Rect::of(&ui.get("create.source.out"));
    let target = ((1.42 - n.view) / crate::create::SPAN) as f32;
    let from = out.centre();
    let dx = (target - ((n.end - n.view) / crate::create::SPAN) as f32) * wave.w;
    ui.drag_points(from, slint::LogicalPosition::new(from.x + dx, from.y));
    let n = crate::create::note(&ui.app).unwrap();
    assert!((n.end - 1.42).abs() < 0.03, "OUT dragged to 1.42 s: {}", n.end);
    assert!(ui.wait_until(5.0, |u| note_state(u) == "DETECTED"), "{}", note_state(&ui));
    assert_eq!(ui.value("create.source.pitch"), "A3 0 ct", "220 Hz");
    assert!(ui.value("create.source.length").contains("0.38 s"), "{}", ui.value("create.source.length"));
    // HEAR: the audition voice plays the note; deck 1 stays where it was.
    let before = snapshot(&ui).decks[0];
    ui.click("create.source.hear");
    assert!(ui.wait_until(2.0, |u| snapshot(u).auditioning), "HEAR plays");
    assert_eq!(ui.get("create.source.hear").accessible_label().map(|s| s.to_string()), Some("HEAR".into()));
    assert!(ui.wait_until(3.0, |u| !snapshot(u).auditioning), "and stops at OUT");
    let after = snapshot(&ui).decks[0];
    assert!(!after.playing && after.track == before.track && (after.position - before.position).abs() < 1e-6, "deck 1 untouched");
    // A semitone up: SET BY YOU.
    ui.click("create.source.note.up");
    assert!(ui.wait_until(2.0, |u| note_state(u) == "SET BY YOU"));
    assert_eq!(ui.value("create.source.pitch"), "Bb3 0 ct");
    ui.click("create.source.cents.down");
    assert_eq!(ui.value("create.source.pitch"), "Bb3 -1 ct", "a cent lower");
    // A typed name.
    ui.click("create.source.type");
    ui.type_text("F#2");
    ui.key(slint::platform::Key::Return);
    assert!(ui.wait_until(2.0, |u| u.value("create.source.pitch") == "F#2 0 ct"), "{}", ui.value("create.source.pitch"));
    // Not a note: nothing changes, the status line says what to type.
    ui.click("create.source.type");
    ui.type_text("H9");
    ui.key(slint::platform::Key::Return);
    assert_eq!(ui.value("create.source.pitch"), "F#2 0 ct");
    assert!(ui.app.ui.get_status_text().contains("Not a note"), "{}", ui.app.ui.get_status_text());
    // OUT moved again: measured anew, the correction stays with the note.
    ui.app.ui.global::<crate::Create>().invoke_nudge("out".into(), -1);
    ui.pump(100);
    assert_eq!(note_state(&ui), "SET BY YOU", "the correction stays");
    assert!(ui.fx.lib().meta(crate::create::META_NOTE).unwrap().is_some_and(|j| j.contains("\"set\"")), "stored");
    // Restart: the note, its correction and the waveform are back.
    let Ui { app, fx, .. } = ui;
    drop(app);
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_set_mode("create".into());
    crate::create::set_view(&ui.app, "melody");
    ui.pump(300);
    assert_eq!(ui.value("create.source.pitch"), "F#2 0 ct", "the readout survives a restart");
    assert_eq!(note_state(&ui), "SET BY YOU");
    assert!(ui.has("create.source.in") && ui.has("create.source.out"));
    assert_eq!(crate::create::note(&ui.app).map(|n| n.track), Some(Some(id)));
}

/// PLAYER picks from the track in Prepare's player (decoded in the
/// background); a note made too short says UNSURE and why.
#[test]
fn a_note_from_the_player_and_a_too_short_one() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.app.ui.invoke_select_section("library".into());
    ui.app.ui.invoke_row_clicked(0, false, false);
    ui.pump(200);
    let current = ui.app.state.borrow().player.current.expect("a track in the player");
    ui.app.ui.invoke_set_mode("create".into());
    crate::create::set_view(&ui.app, "melody");
    assert!(ui.wait_until(3.0, |u| u.get("create.source.player").accessible_enabled() == Some(true)), "PLAYER can be picked");
    assert_eq!(ui.get("create.source.deck1").accessible_enabled(), Some(false), "no deck to pick from");
    ui.click("create.source.player");
    assert_eq!(crate::create::note(&ui.app).and_then(|n| n.track), Some(current));
    assert!(ui.wait_until(20.0, |u| note_state(u) == "DETECTED" || note_state(u) == "UNSURE"), "measured once decoded: {}", note_state(&ui));
    assert_ne!(ui.value("create.source.pitch"), "");
    // OUT right after IN: too short to tell.
    let n = crate::create::note(&ui.app).unwrap();
    let g = ui.app.ui.global::<crate::Create>();
    g.invoke_drag_handle("out".into(), ((n.start + 0.04 - n.view) / crate::create::SPAN) as f32);
    g.invoke_drag_done();
    ui.pump(100);
    assert_eq!(note_state(&ui), "UNSURE");
    assert!(ui.value("create.source.length").contains("too short"), "{}", ui.value("create.source.length"));
}

// ── [72] The note on pads ───────────────────────────────────────────────

/// Picking a note enables the pads (the source note and 7 semitones up),
/// rendered in the background; a pad plays its note on the audition voice
/// at the right pitch, the same buffer every time; the octave switch beyond
/// ±7 semitones shows FAR with the hint; deck 1 is never touched.
#[test]
fn pads_play_the_note_at_other_pitches_and_warn_far_from_it() {
    let ui = Ui::open((1440, 900));
    deck1_at(&ui, 1.04);
    assert_eq!(ui.get("create.notes.pad1").accessible_enabled(), Some(false), "no note: pads off");
    assert!(ui.value("create.notes.pads-hint").starts_with("Pick a note in SOURCE"), "{}", ui.value("create.notes.pads-hint"));
    assert_eq!(ui.get("create.notes.octave").accessible_enabled(), Some(false));
    ui.click("create.source.deck1");
    // OUT before the next click (at 1.5 s): only the 220 Hz tone.
    let g = ui.app.ui.global::<crate::Create>();
    let n = crate::create::note(&ui.app).expect("a note");
    g.invoke_drag_handle("out".into(), ((1.42 - n.view) / crate::create::SPAN) as f32);
    g.invoke_drag_done();
    assert!(ui.wait_until(5.0, |u| note_state(u) == "DETECTED"), "{}", note_state(&ui));
    assert_eq!(ui.get("create.notes.pad1").accessible_enabled(), Some(true), "a note: pads on");
    assert_eq!(ui.value("create.notes.pad1"), "A3 0");
    assert_eq!(ui.value("create.notes.pad8"), "E4 +7");
    assert!(!ui.value("create.notes.pads-hint").contains("FAR"));
    // All eight rendered off the UI thread.
    assert!(
        ui.wait_until(20.0, |u| (0..8).all(|i| crate::create::pad_note(&u.app, i).is_some())),
        "the pads are rendered"
    );
    // The method by the rule: 0.38 s a semitone up is resampled, a fifth
    // up stretched (where the stretcher is built in).
    let (_, m1) = crate::create::pad_note(&ui.app, 1).unwrap();
    assert_eq!(m1, od_engine::noteshift::ShiftMethod::Resample);
    let (e4, m7) = crate::create::pad_note(&ui.app, 7).unwrap();
    if od_engine::keylock::Stretcher::AVAILABLE {
        assert_eq!(m7, od_engine::noteshift::ShiftMethod::Stretch);
    }
    let mono: Vec<f32> = e4.samples.chunks_exact(2).map(|f| (f[0] + f[1]) * 0.5).collect();
    let hz = od_analysis::pitch::detect_pitch(&mono, e4.sample_rate).hz.expect("a pitch");
    let cents = 1200.0 * (hz / od_core::NotePitch::new(64, 0).hz()).log2();
    assert!(cents.abs() < 5.0, "pad 8 sounds E4: {hz:.2} Hz ({cents:+.1} ct)");
    // A pad plays at once, the same buffer as before, beside the paused deck.
    let before = snapshot(&ui).decks[0];
    let (a, _) = crate::create::pad_note(&ui.app, 2).unwrap();
    ui.click("create.notes.pad3");
    assert!(ui.wait_until(2.0, |u| snapshot(u).auditioning), "the pad plays");
    assert!(!ui.app.ui.global::<crate::Create>().get_note().hearing, "HEAR is not lit by a pad");
    assert!(ui.app.ui.global::<crate::Create>().get_pads().row_data(2).unwrap().playing, "pad 3 shows it plays");
    let (b, _) = crate::create::pad_note(&ui.app, 2).unwrap();
    assert!(std::sync::Arc::ptr_eq(&a, &b), "the cached buffer");
    assert!(ui.wait_until(3.0, |u| !snapshot(u).auditioning), "and ends");
    let after = snapshot(&ui).decks[0];
    assert!(!after.playing && (after.position - before.position).abs() < 1e-6, "deck 1 untouched");
    // An octave up: +12 … +19, every pad FAR, with the hint.
    ui.click("create.notes.octave.up");
    assert_eq!(ui.value("create.notes.octave.value"), "+1");
    assert_eq!(ui.value("create.notes.pad1"), "A4 +12 FAR");
    assert!(ui.value("create.notes.pads-hint").contains("FAR") && ui.value("create.notes.pads-hint").contains("C5"), "{}", ui.value("create.notes.pads-hint"));
    let tip = ui.get("create.notes.pad1").accessible_description().unwrap_or_default();
    assert!(tip.contains("Pick a source note nearer"), "{tip}");
    // A FAR pad plays too (rendered when pressed if not yet in).
    ui.click("create.notes.pad1");
    assert!(ui.wait_until(10.0, |u| snapshot(u).auditioning), "a FAR pad plays");
    // An octave down: -12 … -5, FAR up to -8.
    ui.click("create.notes.octave.down");
    ui.click("create.notes.octave.down");
    assert_eq!(ui.value("create.notes.pad1"), "A2 -12 FAR");
    assert_eq!(ui.value("create.notes.pad5"), "Db3 -8 FAR");
    assert_eq!(ui.value("create.notes.pad6"), "D3 -7");
    // Keys: the octave switch stops at ±2.
    for _ in 0..4 {
        ui.click("create.notes.octave.down");
    }
    assert_eq!(ui.value("create.notes.octave.value"), "-2");
}

// ── [73] Key and scale ──────────────────────────────────────────────────

fn set_key(ui: &Ui, track: od_core::TrackId, key: &str) {
    ui.app.state.borrow_mut().lib.set_key(track, od_core::MusicalKey::parse(key)).unwrap();
}

fn pad_names(ui: &Ui) -> Vec<String> {
    (1..=8).map(|i| ui.value(&format!("create.notes.pad{i}")).split(' ').next().unwrap_or_default().to_string()).collect()
}

/// A note from deck 1 plays in deck 2's key (C major): the pads hold the
/// scale from the C below the note, the strip lights root, scale and the
/// notes out of it, a key of the strip plays; SNAP TO SCALE off brings the
/// semitones back. NEXT TRACK is off until the queue holds a track, then
/// takes its key; the root and the scale set by hand say SET BY YOU, OTHER
/// DECK follows the deck again; the choice survives a restart.
#[test]
fn the_key_comes_from_the_other_deck_or_the_next_track_and_fills_pads_and_strip() {
    let ui = Ui::open((1440, 900));
    let (a, b, c) = (ui.fx.tracks[0].0, ui.fx.tracks[1].0, ui.fx.tracks[2].0);
    set_key(&ui, a, "8A");
    set_key(&ui, b, "8B");
    set_key(&ui, c, "11A");
    crate::engine::ensure_engine(&ui.app);
    crate::decks::load_track(&ui.app, 1, b, None);
    assert!(ui.wait_until(30.0, |u| snapshot(u).decks[1].track == Some(b)), "deck 2 loads");
    deck1_at(&ui, 1.04);
    ui.click("create.source.deck1");
    let g = ui.app.ui.global::<crate::Create>();
    let n = crate::create::note(&ui.app).expect("a note");
    g.invoke_drag_handle("out".into(), ((1.42 - n.view) / crate::create::SPAN) as f32);
    g.invoke_drag_done();
    assert!(ui.wait_until(5.0, |u| note_state(u) == "DETECTED"), "{}", note_state(&ui));
    // Deck 2's key: C major, as a note name with the wheel beside.
    assert!(ui.wait_until(3.0, |u| u.value("create.notes.key-from") == "FROM DECK 2"), "{}", ui.value("create.notes.key-from"));
    assert_eq!(ui.value("create.notes.root.value"), "C");
    assert_eq!(ui.value("create.notes.wheel"), "8B");
    assert_eq!(ui.get("create.notes.scale.major").accessible_value().map(|s| s.to_string()), Some("HAPPY".into()));
    assert_eq!(ui.value("create.fits.notes"), "C D E F G A B");
    assert_eq!(ui.value("create.fits.chords"), "C Dm Em F G Am Bdim");
    // Snapped: from A3 the C below, C3 to C4.
    assert_eq!(pad_names(&ui), ["C3", "D3", "E3", "F3", "G3", "A3", "B3", "C4"]);
    assert!(ui.value("create.notes.pad1").ends_with("FAR"), "C3 is 9 semitones down: {}", ui.value("create.notes.pad1"));
    // The strip from C3: the root, the scale and the rest as the key says.
    let key = od_core::ScaleKey::new(0, od_core::Scale::Major);
    let keys: Vec<crate::CreateKeyNote> = g.get_keyboard().iter().collect();
    assert_eq!((keys.len(), keys[0].name.as_str()), (24, "C3"));
    for (i, k) in keys.iter().enumerate() {
        let m = 48 + i as i32;
        let want = if m % 12 == 0 { "root" } else if key.contains(m) { "scale" } else { "out" };
        assert_eq!(k.kind, want, "{}", k.name);
        assert_eq!(k.pad, (48..=60).contains(&m) && key.contains(m), "{} on a pad", k.name);
    }
    assert_eq!(ui.value("create.keyboard.key0"), "C3 root on a pad");
    assert_eq!(ui.value("create.keyboard.key1"), "Db3 out of the scale");
    // A key of the strip (E3) plays on the audition voice.
    let e3 = Rect::of(&ui.get("create.keyboard.key4"));
    ui.click_at(slint::LogicalPosition::new(e3.x + e3.w / 2.0, e3.y + e3.h - 20.0), slint::platform::PointerEventButton::Left);
    assert!(ui.wait_until(10.0, |u| snapshot(u).auditioning), "the key plays");
    assert!(g.get_keyboard().row_data(4).unwrap().playing, "E3 shows it plays");
    assert!(!g.get_note().hearing, "HEAR is not lit by a key");
    // SNAP TO SCALE off: every semitone from the source note again.
    ui.click("create.notes.snap");
    assert!(!crate::create::choice(&ui.app).snap);
    assert_eq!(pad_names(&ui), ["A3", "Bb3", "B3", "C4", "Db4", "D4", "Eb4", "E4"]);
    ui.click("create.notes.snap");
    assert_eq!(pad_names(&ui)[0], "C3");
    // NEXT TRACK: off with nothing next, then the queue's next track.
    assert_eq!(ui.get("create.notes.next-track").accessible_enabled(), Some(false));
    let tip = ui.get("create.notes.next-track").accessible_description().unwrap_or_default();
    assert!(tip.contains("No next track"), "{tip}");
    crate::queue::add(&ui.app, &[c]);
    assert!(ui.wait_until(3.0, |u| u.get("create.notes.next-track").accessible_enabled() == Some(true)), "a next track");
    ui.click("create.notes.next-track");
    assert_eq!(ui.value("create.notes.key-from"), "NEXT TRACK");
    assert_eq!(ui.value("create.notes.root.value"), "F#", "11A is F# minor");
    assert_eq!(crate::create::choice(&ui.app).key, Some(od_core::ScaleKey::new(6, od_core::Scale::Minor)));
    assert_eq!(pad_names(&ui)[0], "F#3", "the F# at or below A3");
    // By hand: a semitone up, then a mode under MORE.
    ui.click("create.notes.root.up");
    assert_eq!((ui.value("create.notes.root.value").as_str(), ui.value("create.notes.key-from").as_str()), ("G", "SET BY YOU"));
    ui.click("create.notes.scale.more");
    ui.click("create.notes.scale.dorian");
    assert_eq!(crate::create::choice(&ui.app).key, Some(od_core::ScaleKey::new(7, od_core::Scale::Dorian)));
    assert!(!ui.has("create.notes.wheel"), "a mode has no wheel label");
    assert_eq!(ui.value("create.fits.key"), "G DORIAN");
    // OTHER DECK: deck 2's key again.
    ui.click("create.notes.from-deck");
    assert_eq!((ui.value("create.notes.key-from").as_str(), ui.value("create.notes.root.value").as_str()), ("FROM DECK 2", "C"));
    // Kept: a mode by hand, the hints and SNAP TO SCALE off.
    ui.click("create.notes.scale.lydian");
    ui.click("create.notes.snap");
    ui.click("create.notes.hints");
    assert!(ui.get("create.notes.scale.major").accessible_value().is_none_or(|v| v.is_empty()), "no HAPPY without the hints");
    let kept = crate::create::choice(&ui.app);
    assert_eq!((kept.from.as_str(), kept.snap, kept.hints), ("set", false, false));
    let Ui { app, fx, .. } = ui;
    drop(app);
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_set_mode("create".into());
    crate::create::set_view(&ui.app, "melody");
    ui.pump(300);
    assert_eq!(crate::create::choice(&ui.app), kept, "the key survives a restart");
    assert_eq!(ui.value("create.notes.key-from"), "SET BY YOU");
    assert_eq!(ui.value("create.fits.key"), "C LYDIAN");
}

// ── [74] Arrange ────────────────────────────────────────────────────────

fn steps(ui: &Ui) -> Vec<String> {
    (1..=32).map(|i| ui.value(&format!("create.arrange.step{i}"))).collect()
}

fn notes_of(ui: &Ui) -> Vec<Option<i32>> {
    crate::create::melody(&ui.app).steps.iter().map(|s| s.map(|s| s.note)).collect()
}

/// Typed notes fill the strip from step 1 (rests and holds too) and RESULT
/// counts them; what is not a note is reported beside the field and the
/// rest stays; a step is selected, moved (key and arrows), changed and
/// deleted; CLEAR empties it and UNDO brings it back; the grid sets the
/// bars; all of it survives a restart.
#[test]
fn a_typed_melody_fills_the_strip_and_takes_edits() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("create".into());
    crate::create::set_view(&ui.app, "melody");
    ui.pump(300);
    assert_eq!(ui.value("create.arrange.count"), "EMPTY");
    assert!(ui.value("create.result.summary").starts_with("No melody yet"));
    assert_eq!(ui.get("create.arrange.play").accessible_enabled(), Some(false), "nothing to play");
    assert_eq!(ui.get("create.arrange.clear").accessible_enabled(), Some(false));
    assert!(ui.value("create.arrange.hint").contains("replaces the strip from step 1"), "{}", ui.value("create.arrange.hint"));
    ui.click("create.arrange.type");
    ui.type_text("A3 C4 E4 D4 - Bb3 _ H4 G3");
    ui.key(slint::platform::Key::Return);
    let s = steps(&ui);
    assert_eq!(&s[..4], ["A3", "C4", "E4", "D4"]);
    assert_eq!((s[4].as_str(), s[5].as_str(), s[6].as_str()), ("empty", "Bb3", "held"));
    assert_eq!((s[7].as_str(), s[8].as_str()), ("empty", "G3"), "H4 is not a note: its step stays a rest, G3 keeps its place");
    assert!(s[9..].iter().all(|v| v == "empty"));
    assert!(ui.value("create.arrange.hint").starts_with("Not read: H4"), "{}", ui.value("create.arrange.hint"));
    assert_eq!(ui.value("create.arrange.count"), "6 NOTES · 2 BARS", "9 steps of 1/8");
    assert!(ui.value("create.result.summary").starts_with("6 notes · 2 bars · "), "{}", ui.value("create.result.summary"));
    assert!(ui.value("create.result.summary").ends_with(" BPM · 6 cues"), "{}", ui.value("create.result.summary"));
    assert_eq!(ui.value("create.result.check"), "Stick formats: 6 of 32 hot cues · fits", "said before any render");
    assert_eq!(ui.value("create.arrange.steps"), "A3 C4 E4 D4 - Bb3 _ - G3");
    for id in ["create.result.send", "create.result.render", "create.result.to-playlist"] {
        assert_eq!(ui.get(id).accessible_enabled(), Some(false), "{id} comes later");
    }
    // The grid: 1/4 makes it 3 bars, 1/16 one.
    ui.click("create.arrange.grid.4");
    assert_eq!(ui.value("create.arrange.count"), "6 NOTES · 3 BARS");
    ui.click("create.arrange.grid.16");
    assert_eq!(ui.value("create.arrange.count"), "6 NOTES · 1 BAR");
    ui.click("create.arrange.grid.8");
    // Select C4; the edit keys wake up; it moves right (swapped with E4).
    assert_eq!(ui.get("create.arrange.right").accessible_enabled(), Some(false), "nothing selected");
    ui.click("create.arrange.step2");
    assert!(ui.value("create.arrange.step2").ends_with("selected"));
    ui.click("create.arrange.right");
    assert_eq!(&steps(&ui)[..3], ["A3", "E4", "C4 selected"]);
    // The keys: Left moves it back, Up a scale step (no key: a semitone),
    // Shift+Right selects the step after it, Delete empties that.
    ui.key(slint::platform::Key::LeftArrow);
    assert_eq!(notes_of(&ui)[1], Some(60), "C4 back on step 2");
    ui.key(slint::platform::Key::UpArrow);
    assert_eq!(notes_of(&ui)[1], Some(61));
    ui.click("create.arrange.down");
    assert_eq!(notes_of(&ui)[1], Some(60));
    ui.key(slint::platform::Key::RightArrow);
    assert_eq!(notes_of(&ui)[2], Some(60), "moved right again");
    ui.key(slint::platform::Key::LeftArrow);
    ui.key(slint::platform::Key::Delete);
    assert_eq!(notes_of(&ui)[1], None, "deleted");
    assert_eq!(ui.value("create.arrange.count"), "5 NOTES · 2 BARS");
    // CLEAR, then UNDO.
    assert_eq!(ui.get("create.arrange.undo").accessible_enabled(), Some(false));
    ui.click("create.arrange.clear");
    assert!(steps(&ui).iter().all(|v| v == "empty"));
    assert!(ui.value("create.result.summary").starts_with("No melody yet"));
    ui.click("create.arrange.undo");
    assert_eq!(ui.value("create.arrange.count"), "5 NOTES · 2 BARS", "back");
    assert_eq!(ui.get("create.arrange.undo").accessible_enabled(), Some(false), "once");
    // Typing again replaces it; UNDO brings the old one back.
    ui.click("create.arrange.type");
    ui.app.ui.global::<crate::Create>().invoke_type_steps("F4 F4".into());
    assert_eq!(ui.value("create.arrange.count"), "2 NOTES · 1 BAR");
    ui.click("create.arrange.undo");
    assert_eq!(ui.value("create.arrange.count"), "5 NOTES · 2 BARS");
    ui.click("create.arrange.loop");
    ui.click("create.arrange.grid.16");
    let kept = crate::create::melody(&ui.app);
    assert!(kept.looped);
    assert!(ui.fx.lib().meta(crate::create::META_MELODY).unwrap().is_some(), "stored");
    // Restart: the strip, the grid and LOOP are back.
    let Ui { app, fx, .. } = ui;
    drop(app);
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_set_mode("create".into());
    crate::create::set_view(&ui.app, "melody");
    ui.pump(300);
    assert_eq!(crate::create::melody(&ui.app), kept, "the strip survives a restart");
    assert_eq!(ui.value("create.arrange.count"), "5 NOTES · 1 BAR");
    assert_eq!(ui.value("create.arrange.step1"), "A3");
}

/// A note picked from deck 1 with its pads measured and rendered.
fn picked_on_deck1(ui: &Ui) {
    deck1_at(ui, 1.04);
    ui.click("create.source.deck1");
    let g = ui.app.ui.global::<crate::Create>();
    let n = crate::create::note(&ui.app).expect("a note");
    g.invoke_drag_handle("out".into(), ((1.42 - n.view) / crate::create::SPAN) as f32);
    g.invoke_drag_done();
    assert!(ui.wait_until(5.0, |u| note_state(u) == "DETECTED"), "{}", note_state(ui));
    assert!(ui.wait_until(20.0, |u| (0..8).all(|i| crate::create::pad_note(&u.app, i).is_some())), "the pads are rendered");
}

/// REC with nothing playing fills step by step from the cursor (pads and
/// keys); with deck 1 playing a pad lands on the nearest step of its grid,
/// and the deck plays on untouched; again disarms it.
#[test]
fn rec_fills_steps_one_by_one_or_on_the_master_grid() {
    let ui = Ui::open((1440, 900));
    let id = ui.fx.tracks[0].0;
    super::perform_craft::with_grid(&ui, id);
    picked_on_deck1(&ui);
    ui.click("create.notes.rec");
    assert!(ui.app.ui.global::<crate::Create>().get_rec());
    assert_eq!(ui.get("create.notes.rec").accessible_label().map(|s| s.to_string()), Some("REC ON".into()), "armed, with the word");
    assert!(ui.value("create.arrange.step1").ends_with("records next"));
    assert!(ui.value("create.arrange.hint").contains("now step 1"), "{}", ui.value("create.arrange.hint"));
    let pads: Vec<i32> = (0..3).map(|i| crate::create::pad_targets(crate::create::note(&ui.app).unwrap().pitch().unwrap(), 0, crate::create::choice(&ui.app).pad_key())[i]).collect();
    ui.click("create.notes.pad1");
    ui.click("create.notes.pad3");
    ui.click("create.notes.pad2");
    assert_eq!(&notes_of(&ui)[..4], [Some(pads[0]), Some(pads[2]), Some(pads[1]), None], "step by step");
    assert!(ui.value("create.arrange.step4").ends_with("records next"));
    // A tapped step moves the cursor there.
    ui.click("create.arrange.step10");
    ui.click("create.notes.pad1");
    assert_eq!(notes_of(&ui)[9], Some(pads[0]));
    // Deck 1 plays on a grid: the cursor goes, pads land on the grid.
    ui.click("create.arrange.clear");
    assert!(snapshot(&ui).decks[0].track == Some(id) && snapshot(&ui).decks[0].beat_in_bar > 0, "deck 1 with a grid");
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Play);
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].playing), "deck 1 plays");
    assert!(ui.wait_until(2.0, |u| u.app.ui.global::<crate::Create>().get_rec_mode() == "grid"), "REC follows the master grid");
    assert!(ui.value("create.arrange.hint").contains("nearest step of the master grid"));
    let before = snapshot(&ui);
    let origin = crate::arrange::bar_start(before.decks[0].beat, before.decks[0].beat_in_bar);
    ui.click("create.notes.pad1");
    let first = notes_of(&ui).iter().position(Option::is_some).expect("a note on the grid");
    let want = crate::arrange::nearest_step(before.decks[0].beat, origin, crate::arrange::Grid::Eighth);
    let near = |a: usize, b: usize| (a as i64 - b as i64).rem_euclid(32).min((b as i64 - a as i64).rem_euclid(32)) <= 1;
    assert!(near(first, want), "the nearest step: {first}, near {want}");
    // Two beats later: four steps on (1/8), not the next one.
    let at = snapshot(&ui).decks[0].beat;
    assert!(ui.wait_until(3.0, |u| snapshot(u).decks[0].beat >= at + 2.0));
    let later = snapshot(&ui).decks[0].beat;
    ui.click("create.notes.pad2");
    let second = (0..32).find(|&i| i != first && notes_of(&ui)[i].is_some()).expect("a second note");
    let want = crate::arrange::nearest_step(later, origin, crate::arrange::Grid::Eighth);
    assert!(near(second, want), "{second} near {want}");
    let s = snapshot(&ui);
    assert!(s.decks[0].playing && s.decks[0].track == Some(id), "deck 1 plays on");
    ui.click("create.notes.rec");
    assert!(!ui.app.ui.global::<crate::Create>().get_rec());
    let n = crate::create::melody(&ui.app).notes();
    ui.click("create.notes.pad1");
    assert_eq!(crate::create::melody(&ui.app).notes(), n, "disarmed: the pads only play");
}

/// PLAY sends the notes in order to the audition voice on the running
/// clock, lights the step that sounds, ends after the last bar (LOOP goes
/// round) and stops at once when pressed again.
#[test]
fn play_sends_the_steps_in_order_and_lights_the_step() {
    let ui = Ui::open((1440, 900));
    picked_on_deck1(&ui);
    ui.app.ui.global::<crate::Create>().invoke_type_steps("A3 B3 - C4 D4".into());
    assert!(ui.wait_until(20.0, |u| [57, 59, 60, 62].iter().all(|&t| crate::create::rendered(&u.app, t))), "the strip's notes are rendered");
    crate::create::take_sent(&ui.app);
    ui.click("create.arrange.play");
    assert!(ui.app.ui.global::<crate::Create>().get_playing());
    assert_eq!(ui.get("create.arrange.play").accessible_label().map(|s| s.to_string()), Some("STOP".into()));
    let mut lit = std::collections::BTreeSet::new();
    let mut heard = false;
    let done = ui.wait_until(10.0, |u| {
        lit.extend((1..=8).filter(|i| u.value(&format!("create.arrange.step{i}")).ends_with("playing")));
        heard |= snapshot(u).auditioning;
        !u.app.ui.global::<crate::Create>().get_playing()
    });
    assert!(done, "PLAY ends after the bar");
    assert!(heard, "on the audition voice");
    assert_eq!(crate::create::take_sent(&ui.app), [(0, 57), (1, 59), (3, 60), (4, 62)], "in order, on their steps");
    assert!(lit.len() >= 4 && lit.contains(&1), "the playing step lights: {lit:?}");
    assert!(!snapshot(&ui).decks[0].playing, "deck 1 untouched");
    // LOOP goes round until stopped; a stop is at once.
    ui.click("create.arrange.loop");
    ui.click("create.arrange.play");
    assert!(ui.wait_until(8.0, |u| crate::create::take_sent(&u.app).iter().filter(|s| s.0 == 0).count() >= 1 && u.app.ui.global::<crate::Create>().get_playing()));
    ui.pump(2500);
    assert!(ui.app.ui.global::<crate::Create>().get_playing(), "still going round");
    ui.click("create.arrange.play");
    assert!(!ui.app.ui.global::<crate::Create>().get_playing());
    assert!(ui.wait_until(1.0, |u| !snapshot(u).auditioning), "stopped at once");
    assert!(steps(&ui).iter().all(|v| !v.ends_with("playing")));
}

// ── [75] Render ─────────────────────────────────────────────────────────

fn label(ui: &Ui, id: &str) -> String {
    ui.get(id).accessible_label().map(|s| s.to_string()).unwrap_or_default()
}

/// The file rendered last: its frames, rate, grid and cues in the library.
fn rendered_file(ui: &Ui) -> (crate::create_render::Made, u32, u32, od_core::Beatgrid, Vec<od_core::Cue>) {
    let made = crate::create::made(&ui.app).expect("a render");
    let r = hound::WavReader::open(&made.path).expect("a WAV file");
    let (frames, rate) = (r.duration(), r.spec().sample_rate);
    let lib = ui.fx.lib();
    let grid = lib.beatgrid(made.track).unwrap().expect("a beatgrid");
    let cues = lib.cues(made.track).unwrap();
    (made, frames, rate, grid, cues)
}

/// RESULT says what the stick formats take before anything is rendered;
/// RENDER writes a new WAV file beside the library and adds it as a track
/// with a locked grid at the chosen tempo and one hot cue per note (named
/// by the note), then reads READY until the notes, layout or tempo change;
/// MELODY keeps the strip in time, SINGLE NOTES puts each distinct note on
/// its own bar, and the track's tempo can be chosen. The source file is
/// never touched.
#[test]
fn render_writes_a_new_track_with_grid_and_cues_and_leaves_the_source() {
    let ui = Ui::open((1440, 900));
    let id = ui.fx.tracks[0].0;
    super::perform_craft::with_grid(&ui, id);
    let source = ui.fx.lib().track(id).unwrap().path;
    let bytes = std::fs::read(&source).unwrap();
    let mtime = std::fs::metadata(&source).unwrap().modified().unwrap();
    picked_on_deck1(&ui);
    let g = ui.app.ui.global::<crate::Create>();
    // Nothing on the strip: nothing to render, no check yet.
    assert!(!ui.has("create.result.check"));
    assert_eq!(ui.get("create.result.render").accessible_enabled(), Some(false));
    g.invoke_type_steps("A3 C4 _ - E4 A3".into());
    ui.pump(100);
    assert_eq!(ui.value("create.result.check"), "Stick formats: 4 of 32 hot cues · fits", "before the render");
    let bpm = g.get_render().bpm.to_string();
    assert_eq!(ui.value("create.result.summary"), format!("4 notes · 1 bar · {bpm} BPM · 4 cues"));
    assert!(ui.get("create.result.melody").accessible_label().is_some());
    assert_eq!(g.get_render().layout, "melody", "MELODY by default");
    assert_eq!(label(&ui, "create.result.render"), "RENDER");
    assert_eq!(ui.get("create.result.render").accessible_enabled(), Some(true));
    // MELODY at the master tempo.
    ui.click("create.result.render");
    assert!(ui.wait_until(20.0, |u| label(u, "create.result.render") == "READY"), "READY: {}", label(&ui, "create.result.render"));
    assert_eq!(g.get_render().state, "ready");
    let (made, frames, rate, grid, cues) = rendered_file(&ui);
    let master: f64 = bpm.parse().unwrap();
    assert!(made.path.starts_with(ui.fx.root.join("library/created")), "{}", made.path.display());
    assert!(made.title.ends_with(&format!("(Melody {} BPM)", crate::create_render::tempo(master))), "{}", made.title);
    let at = |beat: f64| crate::create_render::seconds(beat, crate::create_render::tempo(master));
    assert_eq!(frames as usize, (at(4.0) * f64::from(rate)).round() as usize, "one bar at the master tempo");
    assert!(grid.locked && (grid.first_bpm().unwrap() - crate::create_render::tempo(master)).abs() < 1e-9);
    let named: Vec<(String, f64)> = cues.iter().map(|c| (c.label.clone(), c.start)).collect();
    assert_eq!(named.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(), ["A3", "C4", "E4", "A3"], "one cue per note, in order");
    for ((_, start), beat) in named.iter().zip([0.0, 0.5, 2.0, 2.5]) {
        assert!((start - at(beat)).abs() < 1e-6, "on its step: {start} vs beat {beat}");
    }
    assert!(cues.iter().enumerate().all(|(i, c)| c.kind == od_core::CueKind::Hot(i as u8)), "A, B, C, D");
    let t = ui.fx.lib().track(made.track).unwrap();
    assert_eq!(t.tags.title, made.title, "a library track");
    // READY again only says where it is.
    ui.click("create.result.render");
    assert_eq!(crate::create::made(&ui.app).unwrap().track, made.track, "no second file");
    // SINGLE NOTES: READY goes, three distinct notes on three bars.
    ui.click("create.result.single-notes");
    assert_eq!(label(&ui, "create.result.render"), "RENDER", "another file now");
    assert_eq!(ui.value("create.result.check"), "Stick formats: 3 of 32 hot cues · fits");
    assert_eq!(ui.value("create.result.summary"), format!("3 notes · 3 bars · {bpm} BPM · 3 cues"));
    // At the track's tempo (240 BPM).
    ui.click("create.result.tempo.track");
    assert_eq!(g.get_render().bpm, "240.0");
    ui.click("create.result.render");
    assert!(ui.wait_until(20.0, |u| label(u, "create.result.render") == "READY"));
    let (single, frames, rate, grid, cues) = rendered_file(&ui);
    assert_ne!(single.track, made.track);
    assert_ne!(single.path, made.path);
    assert_eq!(frames as usize, 3 * rate as usize, "three bars at 240 BPM: 3 s");
    assert_eq!(cues.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["A3", "C4", "E4"], "low to high");
    for (i, c) in cues.iter().enumerate() {
        let b = grid.nearest_beat(c.start).unwrap();
        assert!((grid.beats[b].time - c.start).abs() < 1e-9 && grid.beats[b].bar_position == 1, "cue {i} on a downbeat");
        assert!((c.start - i as f64).abs() < 1e-9, "bar {i}");
    }
    // The same notes in another order are the same SINGLE NOTES file;
    // another note is not: READY goes.
    g.invoke_type_steps("E4 C4 A3".into());
    assert_eq!(label(&ui, "create.result.render"), "READY");
    g.invoke_type_steps("A3 C4 G4".into());
    assert_eq!(label(&ui, "create.result.render"), "RENDER");
    // The choices are kept; the source file is untouched.
    assert!(ui.fx.lib().meta(crate::create::META_RENDER).unwrap().is_some_and(|j| j.contains("single") && j.contains("track")));
    assert_eq!(std::fs::read(&source).unwrap(), bytes, "the source's bytes");
    assert_eq!(std::fs::metadata(&source).unwrap().modified().unwrap(), mtime, "and its time");
    assert!(ui.fx.lib().track(id).unwrap().path == source);
}

// ── [76] Into the set ───────────────────────────────────────────────────

/// Four notes picked and rendered from deck 1's track: the file READY.
fn rendered(ui: &Ui) -> crate::create_render::Made {
    super::perform_craft::with_grid(ui, ui.fx.tracks[0].0);
    picked_on_deck1(ui);
    ui.app.ui.global::<crate::Create>().invoke_type_steps("A3 C4 _ - E4 A3".into());
    ui.pump(100);
    ui.click("create.result.render");
    assert!(ui.wait_until(20.0, |u| label(u, "create.result.render") == "READY"), "READY: {}", label(ui, "create.result.render"));
    crate::create::made(&ui.app).expect("a render")
}

/// Titles of the browser's list, in order.
fn list_titles(ui: &Ui) -> Vec<String> {
    ui.app.ui.get_tracks().iter().map(|r| r.title.to_string()).collect()
}

/// CREATED in the browser tree lists the tracks rendered from CREATE and
/// nothing else, sorts like any list and loads onto a deck; a comment
/// edited by hand does not take a track out of it.
#[test]
fn the_created_list_shows_rendered_tracks_only() {
    let ui = Ui::open((1440, 900));
    assert_eq!(ui.app.ui.get_created_count(), 0, "nothing rendered yet");
    let made = rendered(&ui);
    assert!(ui.wait_until(5.0, |u| u.app.ui.get_created_count() == 1), "counted in the tree");
    // A second render, then an ordinary track tagged like a rendered one.
    ui.click("create.result.single-notes");
    ui.click("create.result.render");
    assert!(ui.wait_until(20.0, |u| label(u, "create.result.render") == "READY"));
    let single = crate::create::made(&ui.app).unwrap();
    {
        let mut lib = ui.fx.lib();
        let mut tags = lib.track(ui.fx.tracks[1].0).unwrap().tags;
        tags.comment = "CREATE: A3 C4".into();
        lib.set_tags(ui.fx.tracks[1].0, tags).unwrap();
        let mut tags = lib.track(made.track).unwrap().tags;
        tags.comment = "edited by hand".into();
        lib.set_tags(made.track, tags).unwrap();
    }
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.pump(100);
    let row = ui.scroll_into_view("browser.tree", |u| u.find("browser.tree.created")).expect("CREATED in the tree");
    ui.click_el(&row);
    assert!(ui.wait_until(5.0, |u| u.app.ui.get_node_kind() == "created"));
    assert_eq!(ui.app.ui.get_selected_node(), crate::browser::CREATED);
    assert_eq!(ui.app.ui.get_node_title(), "Created");
    assert_eq!(ui.app.ui.get_created_count(), 2);
    let mut both = vec![made.title.clone(), single.title.clone()];
    let mut shown = list_titles(&ui);
    shown.sort();
    both.sort();
    assert_eq!(shown, both, "the rendered tracks only");
    // Sortable like any list.
    ui.app.ui.invoke_sort("title".into());
    ui.pump(100);
    let sorted = list_titles(&ui);
    ui.app.ui.invoke_sort("title".into());
    ui.pump(100);
    let mut reversed = list_titles(&ui);
    reversed.reverse();
    assert_eq!(sorted, reversed, "sorted by title, both ways");
    // Loads onto a deck.
    let first = list_titles(&ui)[0].clone();
    let id = if first == made.title { made.track } else { single.track };
    // The row menu's "Load to deck › Deck 2" (submenus open on hover, which
    // the testing backend times poorly under load).
    ui.app.ui.invoke_row_menu(0, "deck:2".into());
    assert!(
        ui.wait_until(10.0, |u| crate::engine::snapshot(&u.app).is_some_and(|s| s.decks[1].track == Some(id))),
        "on deck 2"
    );
}

/// TO PLAYLIST waits for a render; then it offers the browser's own
/// playlists (the one added to last inline) or a new playlist named after
/// the file. Each is one undo step.
#[test]
fn to_playlist_adds_to_an_existing_or_a_new_playlist_undoably() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("create".into());
    crate::create::set_view(&ui.app, "melody");
    ui.pump(100);
    assert_eq!(ui.get("create.result.to-playlist").accessible_enabled(), Some(false), "nothing rendered");
    crate::create::to_playlist(&ui.app, ui.fx.playlist.0 as i32);
    assert!(ui.app.ui.get_status_text().contains("RENDER first"), "{}", ui.app.ui.get_status_text());
    let made = rendered(&ui);
    assert_eq!(ui.get("create.result.to-playlist").accessible_enabled(), Some(true), "READY");
    let entries = |ui: &Ui, n: od_library::NodeId| -> Vec<od_core::TrackId> {
        ui.fx.lib().entries(n).unwrap_or_default().iter().map(|e| e.track).collect()
    };
    // An existing playlist, from the menu.
    let before = entries(&ui, ui.fx.playlist);
    // The browser's playlists, in the submenu's model; picking one runs
    // `to-playlist` with its id (the submenu itself opens on hover, which
    // the testing backend times poorly under load).
    let g = ui.app.ui.global::<crate::Create>();
    let warm = g.get_playlists().iter().find(|p| p.name == "Warm-up").expect("Warm-up offered");
    assert_eq!(warm.id as i64, ui.fx.playlist.0);
    g.invoke_to_playlist(warm.id);
    let mut want = before.clone();
    want.push(made.track);
    assert!(ui.wait_until(5.0, |u| entries(u, u.fx.playlist) == want), "added at the end");
    let status = ui.app.ui.get_status_text().to_string();
    assert!(status.starts_with(&format!("Added {} to Warm-up.", made.title)), "{status}");
    assert!(status.contains("No registered stick"), "{status}");
    assert!(!status.contains(&ui.fx.root.to_string_lossy().to_string()), "no local path: {status}");
    ui.app.undo();
    assert_eq!(entries(&ui, ui.fx.playlist), before, "undone");
    // The list added to last comes inline now.
    ui.click("create.result.to-playlist");
    assert!(ui.click_menu_item("Add to “Warm-up”"), "the recent list inline");
    assert!(ui.wait_until(5.0, |u| entries(u, u.fx.playlist) == want));
    ui.app.undo();
    // A new playlist, named after the file.
    let lists = |ui: &Ui| ui.fx.lib().all_nodes().unwrap().len();
    let n = lists(&ui);
    ui.click("create.result.to-playlist");
    assert!(ui.click_menu_item("Add to a new playlist"));
    let node = ui
        .fx
        .lib()
        .all_nodes()
        .unwrap()
        .into_iter()
        .find(|x| x.name == made.title)
        .expect("a new playlist named after the file");
    assert_eq!(node.kind, od_library::NodeKind::Playlist);
    assert_eq!(entries(&ui, node.id), [made.track]);
    assert_eq!(ui.fx.lib().journal(1).unwrap()[0].label, "Add to a new playlist", "one step");
    ui.app.undo();
    assert_eq!(lists(&ui), n, "the new playlist goes with one undo");
    // The track itself stays in the library and in CREATED.
    assert!(ui.fx.lib().track(made.track).is_ok());
    assert_eq!(ui.fx.lib().created_tracks().unwrap().len(), 1);
}

/// Sticks get a rendered track like any other: through a playlist they
/// sync. The status line names the stick; its sync plan holds the track,
/// and after the sync the stick's library has the file with its hot cues
/// and its grid.
#[test]
fn a_rendered_track_in_a_synced_playlist_reaches_the_stick_with_cues_and_grid() {
    let _volumes = volumes_lock();
    let ui = Ui::open((1440, 900));
    let name = "CREATE-STICK";
    let stick = test_env().join("volumes").join(name);
    let _ = std::fs::remove_dir_all(&stick);
    std::fs::create_dir_all(&stick).unwrap();
    let uuid = format!("uuid-{name}-{}", std::process::id());
    od_format_native::write_marker(
        &stick,
        &od_format_native::DeviceMarker {
            format_version: od_format_native::FORMAT_VERSION,
            device_uuid: uuid.clone(),
            name: name.into(),
            created_by: "test".into(),
        },
    )
    .unwrap();
    let (device, set) = {
        let mut lib = ui.fx.lib();
        let device = lib
            .register_device(&uuid, name, od_library::DeviceFormats { club_usb: true, opendjdecks: true, club_usb_compat: false })
            .unwrap();
        let set = lib.create_node(None, od_library::NodeKind::Playlist, "For the stick").unwrap();
        lib.set_device_nodes(device, &[set]).unwrap();
        (device, set)
    };
    ui.app.refresh_library();
    let made = rendered(&ui);
    let lib = ui.fx.lib();
    assert!(lib.device_tracks(device).unwrap().is_empty(), "not on the stick before");
    crate::create::to_playlist(&ui.app, set.0 as i32);
    let status = ui.app.ui.get_status_text().to_string();
    assert!(status.contains(&format!("It goes to {name} with the next sync")), "{status}");
    let plan: Vec<od_core::TrackId> = lib.device_tracks(device).unwrap().iter().map(|t| t.id).collect();
    assert_eq!(plan, [made.track], "in the stick's sync plan");
    let cues = lib.cues(made.track).unwrap();
    assert_eq!(cues.len(), 4);
    ui.app.device_manager.send(od_devices::Command::Refresh);
    assert!(ui.wait_until(10.0, |u| u.app.state.borrow().devices.statuses.iter().any(|s| s.device.id == device && s.volume.is_some())));
    ui.app.device_manager.send(od_devices::Command::SyncNow(device));
    assert!(ui.wait_until(40.0, |_| lib.device(device).unwrap().last_sync.is_some()), "the stick is synced");
    let native = od_format_native::Stick::open(&stick).unwrap().expect("the stick's library");
    let on = native.library.track(made.track).expect("the rendered track on the stick");
    assert_eq!(on.tags.title, made.title);
    assert!(native.resolve(&on.path).exists(), "its file");
    let stick_cues = native.library.cues(made.track).unwrap();
    assert_eq!(
        stick_cues.iter().map(|c| (c.label.as_str(), c.kind)).collect::<Vec<_>>(),
        cues.iter().map(|c| (c.label.as_str(), c.kind)).collect::<Vec<_>>(),
        "its hot cues"
    );
    let grid = native.library.beatgrid(made.track).unwrap().expect("its grid");
    assert_eq!(Some(grid), lib.beatgrid(made.track).unwrap());
    let lists = native.library.all_nodes().unwrap();
    assert!(lists.iter().any(|n| n.name == "For the stick"), "in its playlist");
    // The club stick format gets it too: the stick check reads its cues back.
    let g = ui.app.ui.global::<crate::StickCheck>();
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.app.ui.invoke_open_device(device.0 as i32);
    ui.pump(200);
    ui.click("devices.check");
    assert!(ui.wait_until(20.0, |_| !g.get_running()), "the check finishes");
    assert!(g.get_ready(), "{} | {}", g.get_summary(), g.get_problems());
    let row = g.get_rows().iter().find(|r| r.title == made.title.as_str()).expect("a row for the file");
    assert_eq!(row.file, "ok");
    drop(native);
    let _ = std::fs::remove_dir_all(&stick);
}

// ── [77] Into Perform ───────────────────────────────────────────────────

/// Names in the sampler's MELODY bank, pad by pad ("" = empty).
fn melody_bank(ui: &Ui) -> Vec<String> {
    let base = od_engine::MELODY_BANK * od_engine::SAMPLER_SLOTS;
    ui.app.sampler.slots.borrow()[base..base + od_engine::SAMPLER_SLOTS]
        .iter()
        .map(|s| s.as_ref().map(|s| s.name.clone()).unwrap_or_default())
        .collect()
}

/// Captions of deck 1's pads in its mode.
fn deck1_pads(ui: &Ui) -> Vec<String> {
    crate::decks::pad_rows(&ui.app, 0, &snapshot(ui)).iter().map(|p| p.caption.to_string()).collect()
}

/// Before anything is sent the MELODY pad mode shows where the notes come
/// from and its pads play nothing. SEND TO PERFORM puts the strip's notes
/// (low to high) on pads 1–8 of the sampler's MELODY bank and the octave
/// above on 9–16, and reads SENT until the notes change; the MELODY pads
/// name them, the ‹ › turns to the octave, and a pad starts its note
/// through the sampler on the next beat with QUANT. A controller reaches
/// the mode and its pads through the mapping actions.
#[test]
fn send_to_perform_fills_the_melody_bank_and_its_pad_mode() {
    use od_controller::{ControlAction, DeckAction, PadMode};
    let ui = Ui::open((1440, 900));
    let base = od_engine::MELODY_BANK * od_engine::SAMPLER_SLOTS;
    // Nothing sent: the hint, outlined pads that play nothing.
    crate::engine::ensure_engine(&ui.app);
    ui.app.ui.invoke_set_mode("perform".into());
    crate::decks::set_pad_mode(&ui.app, 0, PadMode::Melody);
    ui.pump(200);
    assert_eq!(ui.value("deck1.pad-mode"), "MELODY");
    assert_eq!(ui.value("deck1.pad-hint"), "Send notes from CREATE");
    assert_eq!(ui.get("deck1.pad.A").accessible_enabled(), Some(false));
    assert_eq!(deck1_pads(&ui), ["1", "2", "3", "4", "5", "6", "7", "8"]);
    crate::decks::pad(&ui.app, 0, 0, true);
    assert!(ui.app.ui.get_status_text().contains("Send notes from CREATE first"), "{}", ui.app.ui.get_status_text());
    // CREATE: a note, a strip, SEND TO PERFORM.
    super::perform_craft::with_grid(&ui, ui.fx.tracks[0].0);
    picked_on_deck1(&ui);
    let g = ui.app.ui.global::<crate::Create>();
    g.invoke_type_steps("E4 A3 C4 A3".into());
    ui.pump(100);
    assert_eq!(label(&ui, "create.result.send"), "SEND TO PERFORM");
    assert_eq!(ui.get("create.result.send").accessible_enabled(), Some(true));
    ui.click("create.result.send");
    assert!(ui.wait_until(20.0, |u| label(u, "create.result.send") == "SENT"), "SENT: {}", label(&ui, "create.result.send"));
    assert_eq!(g.get_render().send, "sent");
    let status = ui.app.ui.get_status_text().to_string();
    assert!(status.starts_with("Sent 3 notes to Perform"), "{status}");
    assert!(!status.contains('/'), "no paths: {status}");
    let mut want = vec![String::new(); 16];
    for (i, (n, up)) in [("A3", "A4"), ("C4", "C5"), ("E4", "E5")].into_iter().enumerate() {
        want[i] = n.into();
        want[8 + i] = up.into();
    }
    assert_eq!(melody_bank(&ui), want, "notes on 1–8, the octave on 9–16");
    assert!(ui.wait_until(5.0, |u| {
        let s = snapshot(u);
        (0..16).all(|p| s.sampler[base + p].loaded == !want[p].is_empty())
    }), "the engine's slots hold them");
    let st = snapshot(&ui);
    assert!(st.sampler[base].mode == od_engine::SampleMode::OneShot && !st.sampler[base].sync);
    // Kept like any loaded sample: a file each, in the sampler's store
    // (written as Perform shows the sampler).
    let folder = ui.fx.root.join("library/created/melody");
    assert!(folder.join("melody-01.wav").exists() && folder.join("melody-06.wav").exists());
    ui.app.ui.invoke_set_mode("perform".into());
    assert!(ui.wait_until(5.0, |u| u.fx.lib().meta(crate::sampler::META_SAMPLER).unwrap().is_some_and(|j| j.contains("melody-06.wav"))));
    // The sampler panel shows the bank under its name.
    crate::sampler::set_bank(&ui.app, od_engine::MELODY_BANK);
    ui.app.ui.set_sampler_panel(true);
    ui.pump(200);
    assert_eq!(ui.get("sampler.bank5").accessible_checked(), Some(true));
    crate::sampler::set_bank(&ui.app, 0);
    // Perform: the note names, the octave behind the ‹ ›.
    ui.pump(200);
    assert!(!ui.has("deck1.pad-hint"));
    assert_eq!(deck1_pads(&ui)[..4], ["A3", "C4", "E4", "4"]);
    assert_eq!(ui.get("deck1.pad.B").accessible_enabled(), Some(true));
    assert_eq!(ui.get("deck1.pad.D").accessible_enabled(), Some(false));
    crate::decks::pad_param(&ui.app, 0, 1);
    assert_eq!(deck1_pads(&ui)[..3], ["A4", "C5", "E5"]);
    crate::decks::pad_param(&ui.app, 0, 1);
    // A pad with QUANT and deck 1 playing: the sampler's slot waits for the
    // next beat, then plays.
    crate::sampler::set_quantize(&ui.app, true);
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Play);
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].playing && snapshot(u).sequencer.quantize == Some(1.0)));
    crate::decks::pad(&ui.app, 0, 1, true);
    assert!(ui.wait_until(5.0, |u| snapshot(u).sampler[base + 1].playing), "C4 plays from the MELODY bank");
    crate::decks::pad(&ui.app, 0, 1, false);
    // A controller: the mode by its mapping action, then a pad.
    crate::decks::set_pad_mode(&ui.app, 1, PadMode::HotCue);
    crate::controller::handle_action(&ui.app, ControlAction::Deck { deck: 2, action: DeckAction::PadMode(PadMode::Melody) });
    assert_eq!(ui.app.decks.borrow().pad_mode[1], PadMode::Melody);
    crate::controller::handle_action(&ui.app, ControlAction::Deck { deck: 2, action: DeckAction::Pad { index: 2, pressed: true, velocity: 127 } });
    assert!(ui.wait_until(5.0, |u| snapshot(u).sampler[base + 2].playing), "E4 plays from deck 2's pad");
    // Other notes: SEND TO PERFORM again.
    ui.app.ui.invoke_set_mode("create".into());
    g.invoke_type_steps("A3 C4 G4".into());
    ui.pump(100);
    assert_eq!(label(&ui, "create.result.send"), "SEND TO PERFORM");
}
