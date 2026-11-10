//! Roadmap [6] and [13]: Simple / Standard / Full deck layouts with hidden
//! groups and saved layouts, and the guided controller setup.

use super::harness::{Fixture, Ui, interactive, volumes_lock};
use slint::ComponentHandle;
use slint::platform::PointerEventButton;

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    assert!(
        ui.wait_until(30.0, |u| u.has("mixer.ch1.fader")),
        "the mixer is up"
    );
}

fn level(ui: &Ui) -> String {
    ui.app
        .ui
        .global::<crate::DeckLayout>()
        .get_level()
        .to_string()
}

/// Clicks an element of an open popup (popups have their own coordinates:
/// the testing API maps the click).
fn pick(ui: &Ui, label: &str) {
    assert!(
        ui.wait_until(10.0, |u| u.has(label)),
        "`{label}` in the menu"
    );
    ui.get(label).mock_single_click(PointerEventButton::Left);
    ui.pump(150);
}

/// The interactive controls of deck `d`: elements a person can press,
/// drag or type into whose label belongs to the deck. A stem key's
/// "separate first" overlay is part of its key, not a control of its own.
fn deck_controls(ui: &Ui, d: usize) -> Vec<String> {
    let prefix = format!("deck{d}.");
    let mut out = Vec::new();
    for (label, es) in ui.labelled() {
        if !label.starts_with(&prefix) || label.ends_with(".request") {
            continue;
        }
        for e in es {
            if interactive(&e) {
                out.push(label.clone());
            }
        }
    }
    out.sort();
    out
}

/// Opens the view menu (if it is not open).
fn open_menu(ui: &Ui) {
    if !ui.has("topbar.layout.2h") {
        ui.click("topbar.layout");
    }
    assert!(
        ui.wait_until(10.0, |u| u.has("topbar.layout.2h")),
        "menu opens"
    );
}

/// Closes the view menu with a click on its own key (outside the popup).
fn close_menu(ui: &Ui) {
    if ui.has("topbar.layout.2h") {
        ui.click("topbar.layout");
        ui.pump(200);
    }
}

fn load_two(ui: &Ui) {
    for i in 0..2 {
        let (id, _) = ui.fx.tracks[i].clone();
        crate::decks::load_track(&ui.app, i, id, None);
    }
    assert!(
        ui.wait_until(60.0, |u| super::flows::deck_views(u)
            .iter()
            .take(2)
            .all(|d| d.loaded)),
        "two decks loaded"
    );
    ui.pump(200);
}

/// A Simple deck has about 25 controls; the groups Simple hides are gone
/// from the element tree (nothing hidden can be found or focused), and
/// Standard brings every one of them back.
#[test]
fn simple_deck_has_about_25_controls_and_hidden_groups_are_gone() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    load_two(&ui);
    assert_eq!(level(&ui), "standard");
    let standard = deck_controls(&ui, 1);
    assert!(
        standard.len() > 30,
        "the standard deck: {} {standard:?}",
        standard.len()
    );
    for l in [
        "deck1.slip",
        "deck1.keylock",
        "deck1.key-sync",
        "deck1.pad-mode",
        "wave.phase",
        "topbar.toggle.fx",
        "fx.channel-fx.ch1",
    ] {
        assert!(ui.has(l), "Standard shows `{l}`");
    }

    open_menu(&ui);
    pick(&ui, "topbar.layout.level.simple");
    assert!(ui.wait_until(10.0, |u| level(u) == "simple"));
    close_menu(&ui);
    let simple = deck_controls(&ui, 1);
    eprintln!(
        "deck 1 controls: standard {}, simple {}: {simple:?}",
        standard.len(),
        simple.len()
    );
    assert!(
        (20..=28).contains(&simple.len()),
        "a Simple deck has about 25 controls, not {}: {simple:?}",
        simple.len()
    );
    // The first tier stays: play, cue, sync, the tempo fader, the pads.
    for l in [
        "deck1.play",
        "deck1.cue",
        "deck1.beat-sync",
        "deck1.pitch",
        "deck1.pad.A",
        "deck1.pad.H",
        "deck1.loop.auto",
    ] {
        assert!(ui.has(l), "Simple keeps `{l}`");
    }
    // Setup keys, the phase strip and the effects are not built at all.
    for l in [
        "deck1.slip",
        "deck1.quantize",
        "deck1.keylock",
        "deck1.key-sync",
        "deck1.key-shift",
        "deck1.pad-mode",
        "deck1.grid-panel",
        "deck1.tempo-mode",
        "deck1.platter",
        "wave.phase",
        "topbar.toggle.fx",
        "fx.channel-fx.ch1",
        "fx.channel-fx.type",
    ] {
        assert!(!ui.has(l), "Simple has no `{l}`, not even an invisible one");
    }
    // The choice is stored.
    assert_eq!(ui.app.state.borrow().settings.perform.layout, "simple");

    // Back to Standard: every group is back.
    open_menu(&ui);
    pick(&ui, "topbar.layout.level.standard");
    assert!(ui.wait_until(10.0, |u| level(u) == "standard" && u.has("deck1.slip")));
    assert_eq!(
        deck_controls(&ui, 1),
        standard,
        "Standard is the deck as before"
    );
}

/// Groups hide as a whole from the view menu (the mixer column too), and a
/// layout can be saved under a name, used again and deleted; it is kept in
/// the settings.
#[test]
fn groups_hide_and_layouts_are_saved_switched_and_deleted() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    assert!(ui.has("deck1.pad.A") && ui.has("deck2.stem.vocal"));
    open_menu(&ui);
    pick(&ui, "topbar.layout.group.pads");
    pick(&ui, "topbar.layout.group.stems");
    pick(&ui, "topbar.layout.group.mixer");
    assert!(ui.wait_until(10.0, |u| !u.has("deck1.pad.A")
        && !u.has("deck2.stem.vocal")
        && !u.has("mixer.ch1.fader")));
    assert!(
        ui.has("deck1.play") && ui.has("deck1.loop.auto"),
        "the other groups stay"
    );
    // Save it as "Warm-up".
    let field = ui.get("topbar.layout.save-name");
    field.mock_single_click(PointerEventButton::Left);
    ui.pump(50);
    ui.type_text("Warm-up");
    pick(&ui, "topbar.layout.save");
    assert!(ui.wait_until(10.0, |u| u.has("topbar.layout.saved.slot1")));
    assert_eq!(ui.value("topbar.layout.saved.slot1"), "Warm-up");
    {
        let p = &ui.app.state.borrow().settings.perform;
        assert_eq!(p.saved_layouts.len(), 1);
        assert_eq!(p.saved_layouts[0].hidden, ["pads", "stems"]);
        assert!(!p.saved_layouts[0].mixer_panel);
        assert_eq!(p.saved_current, "Warm-up");
    }
    // Full shows everything again, with the Tempo FX panel.
    pick(&ui, "topbar.layout.level.full");
    assert!(
        ui.wait_until(10.0, |u| u.has("deck1.pad.A")
            && u.has("mixer.ch1.fader")
            && u.app.ui.get_fx_panel()),
        "full: {} {:?} pads {} mixer {} fx {} panel {} swap {} keys {:?}",
        level(&ui),
        ui.app.state.borrow().settings.perform.hidden,
        ui.has("deck1.pad.A"),
        ui.has("mixer.ch1.fader"),
        ui.app.ui.get_fx_panel(),
        ui.app.ui.get_mixer_panel(),
        ui.has("mixer.swap.low"),
        ui.labelled()
            .keys()
            .filter(|k| k.starts_with("mixer"))
            .cloned()
            .collect::<Vec<_>>()
    );
    // The saved layout comes back in one press.
    pick(&ui, "topbar.layout.saved.slot1");
    assert!(ui.wait_until(10.0, |u| !u.has("deck1.pad.A") && !u.has("mixer.ch1.fader")));
    assert_eq!(level(&ui), "standard");
    // Stored in the library's settings, so it is there after a restart.
    let stored = ui
        .fx
        .lib()
        .meta(crate::settings::META_SETTINGS)
        .unwrap()
        .unwrap();
    assert!(
        crate::settings::Settings::from_json(&stored)
            .perform
            .saved_layouts
            .iter()
            .any(|l| l.name == "Warm-up")
    );
    // Deleted.
    pick(&ui, "topbar.layout.saved.slot1.delete");
    assert!(ui.wait_until(10.0, |u| !u.has("topbar.layout.saved.slot1")));
    assert!(
        ui.app
            .state
            .borrow()
            .settings
            .perform
            .saved_layouts
            .is_empty()
    );
}

/// The first-start guide offers the deck layout once the decks come in
/// (the steps before keep their room for SAMPLE TRACKS and long hints): three keys on a wide
/// window, one key stepping through them below 1480 px, where the hint
/// keeps its room for two lines.
#[test]
fn first_start_offers_the_deck_layout() {
    let _vol = volumes_lock();
    let ui = Ui::start(Fixture::empty(), (1280, 800), false);
    assert!(
        ui.wait_until(20.0, |u| u.has("guide.samples")),
        "the guide band is up"
    );
    assert!(
        !ui.has("guide.layout") && !ui.has("guide.layout.simple"),
        "not on the first steps"
    );
    drop(ui);
    let ui = Ui::open((1600, 900));
    ui.app.ui.global::<crate::FirstStart>().invoke_reopen();
    assert!(
        ui.wait_until(20.0, |u| u.has("guide.layout.simple")),
        "three keys on a wide window"
    );
    ui.click("guide.layout.simple");
    assert!(ui.wait_until(20.0, |u| level(u) == "simple"));
    ui.click("guide.layout.standard");
    assert!(ui.wait_until(20.0, |u| level(u) == "standard"));
    // The longest hint of these steps (about 390 px of text) keeps two
    // lines with room to spare.
    for size in [(1600, 900), (1440, 900), (1280, 800)] {
        ui.resize(size);
        let hint = super::harness::Rect::of(&ui.get("guide.hint"));
        assert!(
            hint.w >= 260.0,
            "the hint keeps its room at {size:?}: {hint:?}"
        );
    }
    assert!(
        ui.wait_until(20.0, |u| u.has("guide.layout")),
        "one key on a laptop"
    );
    ui.click("guide.layout");
    assert!(ui.wait_until(20.0, |u| level(u) == "full"));
    ui.click("guide.layout");
    assert!(ui.wait_until(20.0, |u| level(u) == "simple"));
}

/// The guided controller setup: choose a class, move each control when
/// asked (the matching on-screen control is pointed at), skip what the unit
/// lacks, and save a mapping file that loads and is used from the next
/// start.
#[test]
fn guided_controller_setup_learns_controls_and_saves_a_mapping() {
    use od_controller::monitor::{Direction, MidiMonitor};
    let ui = Ui::open((1440, 900));
    let m = MidiMonitor::new(256);
    ui.app.controller_setup.borrow_mut().monitor = Some(m.clone());
    ui.app.ui.global::<crate::ControllerSetup>().invoke_open();
    assert!(
        ui.wait_until(10.0, |u| u.has("app.controller-setup.class.slot1")),
        "the band is open"
    );
    // Any message names the unit.
    m.record(
        "UNLISTED UNIT",
        Direction::In,
        &[0xB0, 0x7F, 0x00],
        Vec::new(),
    );
    assert!(ui.wait_until(
        10.0,
        |u| u.app.ui.global::<crate::ControllerSetup>().get_port() == "UNLISTED UNIT"
    ));
    ui.click("app.controller-setup.class.slot1");
    ui.click("app.controller-setup.start");
    assert!(ui.wait_until(10.0, |u| u.has("app.controller-setup.prompt")
        && u.value("app.controller-setup.step") == "1/39"));
    assert_eq!(ui.app.ui.get_mode(), "perform", "the setup shows the decks");
    assert_eq!(
        ui.value("app.controller-setup.prompt"),
        "Press PLAY on deck 1"
    );
    // PLAY on screen is pointed at.
    assert!(
        ui.with_prefix("guide.highlight")
            .iter()
            .any(|(_, v, _)| v == "deck1.play")
    );
    m.record(
        "UNLISTED UNIT",
        Direction::In,
        &[0x90, 0x0B, 0x7F],
        Vec::new(),
    );
    m.record(
        "UNLISTED UNIT",
        Direction::In,
        &[0x90, 0x0B, 0x00],
        Vec::new(),
    );
    assert!(
        ui.wait_until(10.0, |u| u
            .value("app.controller-setup.heard")
            .contains("NOTE 0x0B")
            || u.value("app.controller-setup.step") == "2/39"),
        "what arrived is shown: {}",
        ui.value("app.controller-setup.heard")
    );
    assert!(
        ui.wait_until(20.0, |u| u.value("app.controller-setup.step") == "2/39"),
        "taken once it rests"
    );
    // The same key for CUE is refused.
    m.record(
        "UNLISTED UNIT",
        Direction::In,
        &[0x90, 0x0B, 0x7F],
        Vec::new(),
    );
    assert!(ui.wait_until(
        20.0,
        |u| u.app.ui.global::<crate::ControllerSetup>().get_taken() == "1"
    ));
    assert_eq!(ui.value("app.controller-setup.step"), "2/39");
    m.record(
        "UNLISTED UNIT",
        Direction::In,
        &[0x90, 0x0C, 0x7F],
        Vec::new(),
    );
    assert!(ui.wait_until(20.0, |u| u.value("app.controller-setup.step") == "3/39"));
    // BACK and SKIP.
    ui.click("app.controller-setup.back");
    assert!(ui.wait_until(10.0, |u| u.value("app.controller-setup.step") == "2/39"));
    m.record(
        "UNLISTED UNIT",
        Direction::In,
        &[0x90, 0x0D, 0x7F],
        Vec::new(),
    );
    assert!(ui.wait_until(20.0, |u| u.value("app.controller-setup.step") == "3/39"));
    while ui.has("app.controller-setup.skip") {
        ui.click("app.controller-setup.skip");
    }
    assert!(ui.wait_until(10.0, |u| u.has("app.controller-setup.save")));
    pick(&ui, "app.controller-setup.save");
    assert!(ui.wait_until(10.0, |u| u.has("app.controller-setup.saved")));
    let path = ui
        .app
        .controller_setup
        .borrow()
        .saved
        .clone()
        .expect("written");
    assert!(
        path.starts_with(ui.fx.root.join("library/mappings")),
        "{}",
        path.display()
    );
    let mapping = od_controller::Mapping::load(&path).expect("the file loads");
    assert!(mapping.matches_port("UNLISTED UNIT"));
    assert_eq!(mapping.inputs.len(), 2, "PLAY and CUE (deck 1)");
    assert!(
        mapping.inputs.iter().all(|b| b.verify && !b.confirmed),
        "nothing claims to be confirmed"
    );
    assert_eq!(
        ui.app.state.borrow().settings.controller.mapping_file,
        path.display().to_string()
    );
    ui.click("app.controller-setup.close");
    assert!(ui.wait_until(10.0, |u| !u.has("app.controller-setup")));
}
