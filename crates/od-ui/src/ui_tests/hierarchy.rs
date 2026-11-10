//! Roadmap step 3: the top bar's zones, the three tiers of a deck and the
//! first-start guide.

use super::harness::{Fixture, Rect, Ui};
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
