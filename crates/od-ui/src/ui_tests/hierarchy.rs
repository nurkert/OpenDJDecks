//! Roadmap step 3: the top bar's zones, the three tiers of a deck and the
//! first-start guide.

use super::harness::{Fixture, Rect, Ui, volumes_lock};
use crate::settings::{META_SETTINGS, Settings};
use slint::ComponentHandle;

fn rect(ui: &Ui, label: &str) -> Rect {
    Rect::of(&ui.get(label))
}

/// A fixture whose stored settings use `language`.
fn fixture_lang(language: &str) -> Fixture {
    let fx = Fixture::new();
    let mut lib = fx.lib();
    let mut settings = Settings::default();
    settings.view.language = language.into();
    settings.recording.folder = fx.root.join("recordings").display().to_string();
    lib.set_meta(META_SETTINGS, &settings.to_json()).unwrap();
    drop(lib);
    fx
}

/// [5] At the small laptop size the master tempo keeps its full width, the
/// right end of the bar stays inside the window in English and German, and
/// the zones are further apart than the keys inside them.
#[test]
fn top_bar_zones_keep_live_values_whole() {
    for language in ["en", "de"] {
        let ui = Ui::start(fixture_lang(language), (1280, 800), false);
        ui.app.ui.set_mode("perform".into());
        ui.pump(400);
        // "MASTER 000.0 BPM" in Caps is about 106 px wide.
        let status = rect(&ui, "topbar.status");
        assert!(status.w >= 100.0, "{language}: status is {} wide", status.w);
        let clock = rect(&ui, "topbar.clock");
        assert!(
            clock.right() <= 1280.0,
            "{language}: the bar overruns the window: clock ends at {}",
            clock.right()
        );
        let (mixer, record) = (rect(&ui, "topbar.toggle.mixer"), rect(&ui, "topbar.record"));
        assert!(
            status.x - mixer.right() >= 12.0,
            "{language}: panel and status zones touch"
        );
        assert!(
            record.x >= mixer.right() + status.w,
            "{language}: record key not in the status zone"
        );
    }
}

/// [5] The record key is an action in the status zone, separate from the
/// panel switches: it runs a timer and its width does not change when it
/// starts, and the panel switch next to it stays a panel switch.
#[test]
fn record_key_is_an_action_with_a_timer() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.set_mode("perform".into());
    ui.pump(200);
    let before = rect(&ui, "topbar.record");
    assert_eq!(ui.value("topbar.record"), "");
    let panel_was = ui.has("rec.toggle");
    ui.click("topbar.record");
    assert!(
        ui.wait_until(10.0, |u| u.app.ui.get_recording()),
        "the key records"
    );
    assert!(
        ui.wait_until(3.0, |u| u.value("topbar.record") != ""),
        "it shows the time"
    );
    let running = rect(&ui, "topbar.record");
    assert_eq!(running.w, before.w, "the timer keeps the key's width");
    ui.click("topbar.record");
    assert!(ui.wait_until(10.0, |u| !u.app.ui.get_recording()));
    assert_eq!(
        ui.has("rec.toggle"),
        panel_was,
        "starting a recording leaves the panel as it was"
    );
}

/// [5] Undo is an editing action: it is offered in Prepare, not in Perform
/// or Lighting.
#[test]
fn undo_only_where_editing_happens() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.set_undo_label("Rating".into());
    ui.app.ui.set_mode("prepare".into());
    ui.pump(200);
    assert!(ui.has("topbar.undo"), "Prepare offers undo");
    for mode in ["perform", "lighting"] {
        ui.app.ui.set_mode(mode.into());
        ui.pump(200);
        assert!(!ui.has("topbar.undo"), "{mode} offers no undo");
    }
}

/// [8] At the small laptop size the two-deck layout is the full deck: platter
/// and tempo fader are there, SYNC sits with CUE and PLAY and is the strongest
/// key beside them, and the controls use the room down to the bottom of the
/// mixer column.
#[test]
fn full_deck_at_laptop_size_with_transport_tier() {
    for (w, h) in [(1280, 800), (1440, 900)] {
        let ui = Ui::open((w, h));
        ui.click("topbar.mode.perform");
        ui.pump(400);
        assert!(ui.has("deck1.platter"), "{w}: platter");
        assert!(ui.has("deck1.pitch"), "{w}: tempo fader");
        let (sync, cue, play) = (
            rect(&ui, "deck1.beat-sync"),
            rect(&ui, "deck1.cue"),
            rect(&ui, "deck1.play"),
        );
        assert!(sync.bottom() <= cue.y + 1.0, "{w}: SYNC above CUE");
        assert!(cue.bottom() <= play.y + 1.0, "{w}: CUE above PLAY");
        assert!(cue.h >= 48.0 && play.h >= 48.0, "{w}: transport keys {} {}", cue.h, play.h);
        let setup = rect(&ui, "deck1.quantize");
        assert!(sync.h >= setup.h && cue.h > 2.0 * setup.h, "{w}: setup key {} not below SYNC {} / CUE {}", setup.h, sync.h, cue.h);
        assert!(setup.h <= 26.0, "{w}: setup keys are small, {}", setup.h);
        let pad = rect(&ui, "deck1.pad.A");
        assert!(pad.h >= 24.0, "{w}: pad {}", pad.h);
        // The controls reach the deck's bottom edge.
        // Audio (and with it the mixer channels) starts with the first load.
        let first = ui.fx.tracks[0].0;
        crate::decks::load_track(&ui.app, 0, first, None);
        assert!(ui.wait_until(10.0, |u| u.has("mixer.ch1.fader")), "{w}: mixer");
        let fader = rect(&ui, "mixer.ch1.fader");
        assert!(fader.h >= 40.0, "{w}: channel fader {}", fader.h);
        let platter = rect(&ui, "deck1.platter");
        assert!(platter.bottom() >= fader.bottom() - 30.0, "{w}: deck ends high");
    }
}

/// [8] The compact deck stays for the kiosk and four decks; the kiosk keeps
/// its 44 px keys and SYNC in its header.
#[test]
fn kiosk_deck_unchanged() {
    let ui = Ui::start(Fixture::new(), (800, 480), true);
    ui.app.ui.set_browser_open(false);
    ui.pump(400);
    let (sync, play) = (rect(&ui, "deck1.beat-sync"), rect(&ui, "deck1.play"));
    assert!(sync.h >= 44.0 && play.h >= 44.0);
    assert!(sync.y < play.y, "kiosk: SYNC in the header row above the transport");
    assert!(!ui.has("guide.bar"), "no first-start guide on the kiosk");
}

/// [8] An empty hot cue pad is drawn hollow: no fill, a bone-3 outline and
/// the letter in bone-3. A set pad fills in its colour.
#[test]
fn empty_hot_cues_are_hollow() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/perform.slint"),
    )
    .unwrap();
    assert!(src.contains("border-width: root.p.set || root.filled ? 0px : 1px;"));
    assert!(src.contains("background: root.filled ? root.tone : root.p.set ?"));
}

// ── [7] Guided first start ─────────────────────────────────────────────────

/// An empty library with no stick around (see [`volumes_lock`]).
fn start_empty(size: (u32, u32)) -> (std::sync::MutexGuard<'static, ()>, Ui) {
    let volumes = volumes_lock();
    (volumes, Ui::start(Fixture::empty(), size, false))
}

fn guide(ui: &Ui) -> (i32, String, String) {
    let g = ui.app.ui.global::<crate::FirstStart>();
    (g.get_step(), g.get_hint().to_string(), g.get_target().to_string())
}

/// The sodium mark sits on the element the guide names.
fn assert_points_at(ui: &Ui, label: &str) {
    let (_, _, target) = guide(ui);
    assert_eq!(target, label, "guide points elsewhere");
    ui.wait_until(5.0, |u| u.by_value("guide.highlight", label).is_some());
    let (_, mark) = ui
        .by_value("guide.highlight", label)
        .unwrap_or_else(|| {
            let g = ui.app.ui.global::<crate::FirstStart>();
            let marks: Vec<_> = ui
                .with_prefix("guide.")
                .into_iter()
                .map(|(l, v, e)| format!("{l}={v} {:?}", Rect::of(&e)))
                .collect();
            panic!(
                "no highlight on {label}: visible {} target {:?} node {:?} section {:?} rows {} mode {:?} control {:?} window {:?} marks {marks:?}",
                g.get_visible(),
                g.get_target().to_string(),
                ui.app.ui.get_node_kind().to_string(),
                ui.app.ui.get_section().to_string(),
                slint::Model::row_count(&ui.app.ui.get_tracks()),
                ui.app.ui.get_mode().to_string(),
                ui.by_value("browser.empty.add-music", "").map(|(_, e)| Rect::of(&e)),
                ui.app.ui.window().size(),
            )
        });
    let mark = Rect::of(&mark);
    let real = rect(ui, label);
    assert!(
        real.overlap(&mark) > 0.0,
        "highlight on {label} misses the real control: {mark:?} / {real:?}"
    );
}

/// [7] The walk: from an empty library through the sample tracks, analysis,
/// two loaded decks, play and sync to the end, ticking by itself and pointing
/// at the real control for each step, driven only by real input.
#[test]
fn first_start_walks_every_step_on_real_controls() {
    let (_volumes, ui) = start_empty((1440, 900));
    assert!(ui.has("guide.bar"), "the guide opens on an empty library");
    assert_eq!(guide(&ui).0, 0, "step 1 first");
    assert_eq!(guide(&ui).1, "add-music");
    assert_points_at(&ui, "browser.empty.add-music");

    ui.click("guide.samples");
    assert!(
        ui.wait_until(60.0, |u| guide(u).0 >= 1),
        "the sample tracks tick step 1"
    );
    assert_eq!(ui.value("guide.step.0"), "done");
    // Short sample tracks analyse fast: this step can be over by the time the
    // test gets here (steps_tick_in_order_and_stay_ticked covers it).
    if guide(&ui).0 == 1 {
        assert_eq!(guide(&ui).1, "analysing");
        assert_points_at(&ui, "status.activity");
    }

    assert!(
        ui.wait_until(90.0, |u| guide(u).0 >= 2),
        "analysis ticks step 2"
    );
    assert_eq!(guide(&ui).1, "go-perform");
    assert_points_at(&ui, "topbar.mode.perform");

    ui.click("topbar.mode.perform");
    ui.pump(300);
    assert_eq!(guide(&ui).1, "select-track");
    assert_points_at(&ui, "browser.list");

    ui.click("browser.row0");
    assert!(ui.wait_until(3.0, |u| guide(u).1 == "load-1"));
    assert_points_at(&ui, "deck1.load");
    ui.click("deck1.load");
    assert!(
        ui.wait_until(60.0, |u| guide(u).1 == "load-2"),
        "deck 1 loads: {:?}",
        guide(&ui)
    );
    assert_points_at(&ui, "deck2.load");
    ui.click("browser.row1");
    ui.pump(100);
    ui.click("deck2.load");

    assert!(
        ui.wait_until(60.0, |u| guide(u).0 >= 3),
        "two tracks tick step 3: {:?}",
        guide(&ui)
    );
    assert_eq!(guide(&ui).1, "play-1");
    assert_points_at(&ui, "deck1.play");
    ui.click("deck1.play");
    assert!(ui.wait_until(5.0, |u| guide(u).1 == "sync-2"));
    assert_points_at(&ui, "deck2.beat-sync");
    ui.click("deck2.beat-sync");
    assert!(ui.wait_until(5.0, |u| guide(u).1 == "play-2"));
    assert_points_at(&ui, "deck2.play");
    ui.click("deck2.play");
    assert!(
        ui.wait_until(8.0, |u| guide(u).0 == 4),
        "play and sync tick the last step: {:?}",
        guide(&ui)
    );
    assert_eq!(guide(&ui).1, "done");
    for i in 0..4 {
        assert_eq!(ui.value(&format!("guide.step.{i}")), "done", "step {i}");
    }
    assert_points_at(&ui, "global.xfader");

    ui.click("guide.hide");
    assert!(!ui.has("guide.bar"), "HIDE closes the band");
    assert_eq!(
        ui.app.state.borrow().settings.view.first_start,
        "off",
        "and it stays closed"
    );
}

/// [7] An existing library is not interrupted.
#[test]
fn first_start_leaves_an_existing_library_alone() {
    let ui = Ui::open((1440, 900));
    assert!(!ui.has("guide.bar"));
}

/// [7] The band holds at the small laptop size and offers the test tone.
#[test]
fn first_start_fits_and_offers_the_test_tone() {
    let (_volumes, ui) = start_empty((1280, 800));
    for label in ["guide.bar", "guide.samples", "guide.tone", "guide.hide"] {
        assert!(ui.has(label), "{label}");
    }
    let bar = rect(&ui, "guide.bar");
    let last = rect(&ui, "guide.hide");
    assert!(last.right() <= bar.right(), "hide fits at 1280");
    ui.click("guide.tone");
    assert!(ui.app.ui.global::<crate::FirstStart>().get_tone_on());
    ui.click("guide.tone");
    assert!(!ui.app.ui.global::<crate::FirstStart>().get_tone_on());
}

/// [8] / [4] The narrow full deck (1280 px) spells out KEY SYNC and MASTER:
/// both keys keep their full width, and the title keeps room.
#[test]
fn narrow_deck_header_keeps_full_words() {
    let ui = Ui::open((1280, 800));
    ui.click("topbar.mode.perform");
    ui.pump(400);
    let (key_sync, master, title) = (
        rect(&ui, "deck1.key-sync"),
        rect(&ui, "deck1.master"),
        rect(&ui, "deck1.title"),
    );
    assert!(key_sync.w >= 78.0, "KEY SYNC {}", key_sync.w);
    assert!(master.w >= 70.0, "MASTER {}", master.w);
    assert!(title.w >= 60.0, "title {}", title.w);
    assert!(key_sync.right() <= master.x, "KEY SYNC and MASTER side by side");
}

/// The Prepare player without a track: the key row keeps its layout (eight
/// empty hot cue slots, disabled keys at their own width) and no playhead
/// runs through the hint.
#[test]
fn empty_player_keeps_its_key_row() {
    let (_volumes, ui) = start_empty((1440, 900));
    assert!(!ui.app.ui.get_has_track());
    for l in ["A", "H"] {
        assert!(ui.has(&format!("player1.hot-cue.{l}")), "slot {l}");
    }
    let del = rect(&ui, "player1.memory.delete");
    assert!(del.w < 80.0, "DEL keeps its width, {}", del.w);
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/player.slint"),
    )
    .unwrap();
    assert!(src.contains("if root.has-track: Rectangle { x: parent.width / 2 - 1px; width: 2px;"));
    let controls = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/controls.slint"),
    )
    .unwrap();
    assert!(controls.contains("border-width: root.enabled || root.selected || root.filled ? 0px : Theme.hair;"));
}
