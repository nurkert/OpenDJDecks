//! Roadmap 0.9.0 block H through the real window: stems on the headphones
//! from the deck and the Prepare player, your own stem files as a track's
//! parts ([66]); routing in Preferences, the external mixer on the mixer
//! column, split cue on one output, and MIDI clock from the master deck
//! ([67]).

use super::harness::*;
use i_slint_backend_testing::ElementHandle;
use od_core::TrackId;
use od_engine::DeckCommand;
use slint::ComponentHandle;
use slint::platform::PointerEventButton;
use std::sync::{Arc, Mutex};

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
}

fn snapshot(ui: &Ui) -> od_engine::EngineState {
    crate::engine::snapshot(&ui.app).expect("engine runs")
}

fn load(ui: &Ui, deck: usize, id: TrackId) {
    crate::decks::load_track(&ui.app, deck, id, None);
    assert!(
        ui.wait_until(60.0, |u| crate::engine::snapshot(&u.app)
            .is_some_and(|s| s.decks[deck].track == Some(id))),
        "deck {} loads",
        deck + 1
    );
    ui.pump(200);
}

/// A track of its own audio (`pitch`), so no other test shares its stem
/// cache file.
pub(crate) fn own_track(fx: &Fixture, title: &str, pitch: f32) -> TrackId {
    let path = fx.root.join(format!("music/{title}.wav"));
    write_wav(&path, 9.0, pitch);
    let mut lib = fx.lib();
    let id = crate::workers::import_file(&mut lib, &path).unwrap();
    let mut tags = lib.track(id).unwrap().tags;
    tags.title = title.into();
    lib.set_tags(id, tags).unwrap();
    id
}

/// Separated parts for `id` in the stem cache (the vocals are the track).
pub(crate) fn with_stems(ui: &Ui, id: TrackId) {
    let track = ui.fx.lib().track(id).unwrap();
    let audio = od_decode::decode(&track.path, od_decode::Layout::Stereo).unwrap();
    let n = audio.samples.len();
    let stems = od_stems::Stems {
        sample_rate: audio.sample_rate,
        vocals: audio.samples,
        drums: vec![0.0; n],
        bass: vec![0.0; n],
    };
    ui.app.decks.borrow().stem_cache().store(&track.content_hash, &stems).unwrap();
}

fn without_stems(ui: &Ui, id: TrackId) {
    let track = ui.fx.lib().track(id).unwrap();
    let _ = ui.app.decks.borrow().stem_cache().remove(&track.content_hash);
    crate::stems_batch::forget_cached();
}

/// [66] CUE on the stem strip: the stem keys pick parts for the
/// headphones; the deck's levels (the master) do not move, and CUE off
/// ends the audition. Also from a controller (`deck.stem_cue`).
#[test]
fn stem_cue_on_the_deck() {
    let fx = Fixture::new();
    let id = own_track(&fx, "Cue Parts", 547.0);
    let ui = Ui::start(fx, (1440, 900), false);
    perform(&ui);
    with_stems(&ui, id);
    load(&ui, 0, id);
    assert!(ui.wait_until(60.0, |u| snapshot(u).decks[0].stems_available), "parts in");
    assert!(
        ui.wait_until(5.0, |u| u.find("deck1.stem.cue").is_some_and(|e| e.accessible_enabled() == Some(true))),
        "the CUE key on the strip"
    );
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::SetStemLevel { stem: od_engine::Stem::Vocals, level: 0.0 });
    ui.click("deck1.stem.cue");
    assert!(ui.wait_until(5.0, |u| u.get("deck1.stem.cue").accessible_checked() == Some(true)));
    ui.click("deck1.stem.vocal");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].stem_audition == 1), "vocals on the cue");
    assert_eq!(snapshot(&ui).decks[0].stem_levels, [0.0, 1.0, 1.0, 1.0], "the master stays as it was");
    assert!(ui.wait_until(5.0, |u| u.get("deck1.stem.vocal").accessible_description().as_deref() == Some("on the headphones")));
    assert_eq!(ui.get("deck1.stem.drums").accessible_description().as_deref(), Some("not on the headphones"));
    ui.click("deck1.stem.drums");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].stem_audition == 0b11));
    // A controller key toggles a part too.
    crate::controller::handle_action(
        &ui.app,
        od_controller::ControlAction::Deck {
            deck: 0,
            action: od_controller::DeckAction::StemCue { stem: od_controller::StemPart::Drums },
        },
    );
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].stem_audition == 1));
    // CUE off: the audition ends, the keys mute again.
    ui.click("deck1.stem.cue");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[0].stem_audition == 0));
    assert!(ui.wait_until(5.0, |u| u.get("deck1.stem.vocal").accessible_description().as_deref() == Some("muted")));
    without_stems(&ui, id);
}

/// [66] Own stem files in a folder named after the track are its parts:
/// the deck has stems at once (no separation), the strip says OWN and the
/// status line names the folder. Files that do not fit are not used, and
/// the status line says why.
#[test]
fn own_stem_files_are_the_parts() {
    let fx = Fixture::new();
    let good = own_track(&fx, "Own Parts", 557.0);
    let bad = own_track(&fx, "Short Parts", 563.0);
    let folder = fx.root.join("music/Own Parts.stems");
    for (name, pitch) in [("vocals", 300.0), ("drums", 400.0), ("bass", 80.0), ("other", 600.0)] {
        write_wav(&folder.join(format!("{name}.wav")), 9.0, pitch);
    }
    let short = fx.root.join("music/Short Parts stems");
    for (name, secs) in [("vocals", 9.0), ("drums", 4.0), ("bass", 9.0)] {
        write_wav(&short.join(format!("{name}.wav")), secs, 200.0);
    }
    let ui = Ui::start(fx, (1440, 900), false);
    perform(&ui);
    load(&ui, 0, good);
    assert!(ui.wait_until(60.0, |u| snapshot(u).decks[0].stems_available), "own parts in");
    assert!(ui.wait_until(5.0, |u| u.value("deck1.stem.caption") == "OWN"), "the strip says OWN");
    assert!(
        ui.wait_until(5.0, |u| u.app.ui.get_status_text().contains("Own Parts.stems")),
        "the folder named: {}",
        ui.app.ui.get_status_text()
    );
    assert!(crate::decks::own_stems_folder(&ui.app, 0).is_some_and(|f| f.ends_with("Own Parts.stems")));
    load(&ui, 1, bad);
    assert!(
        ui.wait_until(30.0, |u| u.app.ui.get_status_text().contains("not used")),
        "why not: {}",
        ui.app.ui.get_status_text()
    );
    assert!(ui.app.ui.get_status_text().contains("drums is 4.0 s long"), "{}", ui.app.ui.get_status_text());
    assert!(!snapshot(&ui).decks[1].stems_available);
    assert!(crate::decks::own_stems_folder(&ui.app, 1).is_none());
}

/// [66] The player's STEMS tool in Prepare: the parts of the track playing
/// in the player go to the headphones one by one; the source is named.
#[test]
fn player_stems_tool_in_prepare() {
    let fx = Fixture::new();
    let id = own_track(&fx, "Player Parts", 569.0);
    let ui = Ui::start(fx, (1440, 900), false);
    with_stems(&ui, id);
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.pump(300);
    {
        let mut st = ui.app.state.borrow_mut();
        st.player.current = Some(id);
        st.player.cursor = 0.0;
    }
    crate::player::refresh_detail(&ui.app);
    ui.pump(300);
    ui.click("player1.tab.stems");
    assert!(ui.wait_until(5.0, |u| u.has("player1.stems.vocal")), "the STEMS tool");
    assert!(ui.wait_until(5.0, |u| u.value("player1.stems.source") == "PLAY THE TRACK TO PICK ITS PARTS"));
    crate::player::toggle_preview(&ui.app);
    let d = crate::decks::PREVIEW_DECK;
    assert!(ui.wait_until(60.0, |u| crate::engine::snapshot(&u.app).is_some_and(|s| s.decks[d].stems_available)));
    assert!(ui.wait_until(5.0, |u| u.value("player1.stems.source") == "SEPARATED"));
    ui.click("player1.stems.bass");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[d].stem_audition == 4), "bass on the cue");
    assert!(ui.wait_until(5.0, |u| u.get("player1.stems.bass").accessible_checked() == Some(true)));
    ui.click("player1.stems.bass");
    assert!(ui.wait_until(5.0, |u| snapshot(u).decks[d].stem_audition == 0));
    crate::player::toggle_preview(&ui.app);
    without_stems(&ui, id);
}

fn prefs(ui: &Ui) -> crate::PreferencesWindow {
    ui.click("topbar.settings");
    let w = ui
        .app
        .prefs
        .borrow()
        .window
        .as_ref()
        .map(|w| w.clone_strong())
        .expect("Preferences open");
    w.set_page("audio".into());
    w.window().set_size(slint::LogicalSize::new(900.0, 1800.0));
    ui.pump(200);
    w
}

fn prefs_el(w: &crate::PreferencesWindow, label: &str) -> ElementHandle {
    super::harness::find_all(w, label)
        .next()
        .unwrap_or_else(|| panic!("no `{label}` in Preferences"))
}

/// [67] Routing in Preferences › Audio: the master pair, the external mixer
/// with a pair per deck (the mixer column steps aside and names them), and
/// one stereo output with split cue (the engine splits).
#[test]
fn routing_external_mixer_and_split_in_preferences() {
    let ui = Ui::start(Fixture::new(), (1440, 900), false);
    perform(&ui);
    let w = prefs(&ui);
    assert!(prefs_el(&w, "prefs.audio.master-routing").accessible_value().is_some());
    w.invoke_choose("audio.master".into(), 1);
    ui.pump(100);
    assert_eq!(ui.app.state.borrow().settings.audio.master, "34");
    // The external mixer: the strips step aside for the routes.
    assert!(ui.has("mixer.ch1.fader"));
    w.invoke_choose("audio.mixer".into(), 1);
    ui.pump(200);
    assert!(prefs_el(&w, "prefs.audio.deck1-routing").accessible_value().is_some(), "a row per deck");
    w.invoke_choose("audio.deck2".into(), 3);
    assert!(ui.wait_until(5.0, |u| snapshot(u).external_mixer), "the engine bypasses its mixer");
    assert!(ui.wait_until(5.0, |u| u.has("mixer.external")), "the plate shows");
    assert!(!ui.has("mixer.ch1.fader"), "no faders that do nothing");
    assert_eq!(ui.value("mixer.external.deck1"), "OUT 1/2");
    assert_eq!(ui.value("mixer.external.deck2"), "OUT 7/8");
    let routing = crate::routing::routing(&ui.app.state.borrow().settings.audio).unwrap();
    assert_eq!(routing.decks[..2], [Some([0, 1]), Some([6, 7])]);
    w.invoke_choose("audio.mixer".into(), 0);
    assert!(ui.wait_until(5.0, |u| !snapshot(u).external_mixer && u.has("mixer.ch1.fader")));
    // One stereo output: split cue on, the pair rows give way.
    prefs_el(&w, "prefs.audio.split-one").mock_single_click(PointerEventButton::Left);
    assert!(ui.wait_until(5.0, |u| snapshot(u).split_cue), "split cue on");
    ui.pump(200);
    assert!(super::harness::find_all(&w, "prefs.audio.master-routing").next().is_none());
    let r = crate::routing::routing(&ui.app.state.borrow().settings.audio).unwrap();
    assert_eq!((r.master, r.headphones), (None, Some([0, 1])));
    prefs_el(&w, "prefs.audio.split-one").mock_single_click(PointerEventButton::Left);
    assert!(ui.wait_until(5.0, |u| !snapshot(u).split_cue));
    let _ = w.hide();
}

/// [67] MIDI clock from the master deck: Start on its beat, pulses at
/// 24 a beat while it plays, Stop when it stops.
#[test]
fn midi_clock_follows_the_master_deck() {
    let fx = Fixture::new();
    let id = own_track(&fx, "Clock Tempo", 571.0);
    {
        let mut lib = fx.lib();
        let d = lib.track(id).unwrap().duration;
        lib.set_beatgrid(id, od_core::Beatgrid::constant(120.0, 0.0, 1, d)).unwrap();
    }
    let ui = Ui::start(fx, (1440, 900), false);
    perform(&ui);
    let log: Arc<Mutex<Vec<u8>>> = Arc::default();
    let l2 = log.clone();
    crate::routing::start_clock_into(
        "Test Clock",
        Box::new(move |b: &[u8]| l2.lock().unwrap().extend_from_slice(b)),
    );
    load(&ui, 0, id);
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Play);
    assert!(
        ui.wait_until(10.0, |_| log.lock().unwrap().iter().filter(|b| **b == 0xF8).count() >= 48),
        "pulses while the master plays"
    );
    assert_eq!(log.lock().unwrap().iter().filter(|b| **b == 0xFA).count(), 1, "one Start");
    assert!(crate::routing::clock_line().starts_with("MIDI clock on Test Clock"));
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Pause);
    assert!(ui.wait_until(5.0, |_| log.lock().unwrap().contains(&0xFC)), "Stop");
    let n = log.lock().unwrap().len();
    ui.pump(300);
    assert_eq!(log.lock().unwrap().len(), n, "nothing after Stop");
    crate::routing::stop_clock();
    assert_eq!(crate::routing::clock_line(), "MIDI clock off");
}

/// [67] The external mixer's plate takes the strips' place: the decks keep
/// their places and sizes at 1440 × 900 and 1280 × 800.
#[test]
fn the_external_mixer_keeps_the_decks_in_place() {
    for size in [(1440, 900), (1280, 800)] {
        let ui = Ui::start(Fixture::new(), size, false);
        perform(&ui);
        let ids = ["deck1.play", "deck2.play", "deck1.overview", "deck2.overview", "deck1.stem.vocal", "deck2.stem.inst"];
        let before: Vec<Rect> = ids.iter().map(|l| Rect::of(&ui.get(l))).collect();
        ui.app.state.borrow_mut().change_settings(|s| s.audio.mixer = "external".into());
        crate::routing::refresh_mixer(&ui.app);
        assert!(ui.wait_until(5.0, |u| u.has("mixer.external")));
        ui.pump(300);
        for (l, b) in ids.iter().zip(&before) {
            let a = Rect::of(&ui.get(l));
            assert!(
                (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() < 0.5 && (a.w - b.w).abs() < 0.5 && (a.h - b.h).abs() < 0.5,
                "{size:?} {l}: {b:?} → {a:?}"
            );
        }
    }
}
