//! Roadmap 0.8.0 "For the person", lighting: own looks and chases on pads
//! ([15]), the lighting panel in Perform and Club ([17]), AUTO you can
//! steer ([18]) and connecting interfaces ([20]), through the real window.

use super::harness::*;
use od_lighting::{AutoPick, OutputKind, PadAction, PhraseSlot};
use slint::{ComponentHandle, Model};

fn type_into(ui: &Ui, label: &str, text: &str) {
    ui.click(label);
    ui.key(slint::platform::Key::End);
    for _ in 0..24 {
        ui.key(slint::platform::Key::Backspace);
    }
    ui.type_text(text);
    ui.key(slint::platform::Key::Return);
    ui.pump(50);
}

fn g(ui: &Ui) -> crate::Lighting<'_> {
    ui.app.ui.global::<crate::Lighting>()
}

/// A wash par and a mover in two groups.
fn patch_two_groups(ui: &Ui) {
    let gl = g(ui);
    gl.invoke_add_fixtures(
        2,
        "Front".into(),
        "Wash".into(),
        "2".into(),
        "".into(),
        "".into(),
    );
    gl.invoke_add_fixtures(
        3,
        "Spot".into(),
        "Movers".into(),
        "1".into(),
        "".into(),
        "".into(),
    );
    assert!(
        ui.wait_until(3.0, |u| u.app.lighting.borrow().config.patch.fixtures.len()
            == 3)
    );
}

fn rig(ui: &Ui) -> od_lighting::RigConfig {
    od_lighting::RigConfig::load(&crate::lighting::rig_path(&ui.fx.library)).unwrap()
}

/// [15] Make a look and a chase in PADS, with a live colour preview,
/// latch or flash, on two pages; play them; set a group master; all of it
/// is saved with the rig.
#[test]
fn own_looks_and_chases_on_pads_are_saved_with_the_rig() {
    let ui = Ui::open((1440, 900));
    ui.click("topbar.mode.lighting");
    ui.pump(200);
    patch_two_groups(&ui);
    ui.click("lighting.space.show");
    ui.click("lighting.centre.pads");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.pad0")));
    assert!(ui.value("lighting.pad0").contains("1.1"));
    // EDIT, open pad 1.1, make a look.
    ui.click("lighting.pads.edit");
    ui.click("lighting.pad0");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.look.new")));
    ui.click("lighting.look.new");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.look.group0.color0")));
    // Red for the wash at 80 %: the pad's preview follows at once.
    ui.click("lighting.look.group0.color0");
    // "—" (the program decides) sits above 100 %.
    for _ in 0..3 {
        ui.click("lighting.look.group0.level.down");
    }
    assert!(ui.wait_until(3.0, |u| u.value("lighting.look.group0.level") == "80 %"));
    // Movers left to what runs underneath.
    ui.click("lighting.look.group1.clear");
    type_into(&ui, "lighting.look.name", "Red wash");
    ui.pump(100);
    {
        let rt = ui.app.lighting.borrow();
        let views = crate::lighting_pads::pad_views(&rt);
        assert_eq!(views[0].name, "Red wash");
        let c: Vec<slint::Color> = views[0].colors.iter().collect();
        assert_eq!(c.len(), 1, "one colour: the wash");
        assert!(c[0].red() > 150 && c[0].green() < 40, "{:?}", c[0]);
    }
    assert!(ui.value("lighting.look.group1").ends_with("underneath"));
    // Pad 1.2: a flash look.
    ui.click("lighting.pad1");
    ui.click("lighting.look.new");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.look.flash")));
    ui.click("lighting.look.flash");
    // Page 2, pad 2.1: a chase of 1.1 and 1.2.
    ui.click("lighting.page2");
    assert!(ui.wait_until(3.0, |u| u.value("lighting.pad0").contains("2.1")));
    ui.click("lighting.pad0");
    ui.click("lighting.look.new-chase");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().adding_steps));
    ui.click("lighting.page1");
    ui.click("lighting.pad0");
    ui.click("lighting.pad1");
    assert!(
        ui.wait_until(3.0, |u| u.has("lighting.look.step1")),
        "{:?} {:?}",
        ui.app
            .lighting
            .borrow()
            .config
            .settings
            .pads
            .iter()
            .map(|l| (l.pad, l.steps.clone()))
            .collect::<Vec<_>>(),
        ui.app.lighting.borrow().edit_pad
    );
    ui.click("lighting.look.step-beats.up");
    ui.click("lighting.look.add-steps");
    ui.click("lighting.pads.edit");
    // Play: 1.1 latches, a second press lets go; 1.2 flashes while held.
    ui.click("lighting.pad0");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().pads.latched == Some(0)));
    assert!(
        ui.value("lighting.pad0").ends_with(" on"),
        "{}",
        ui.value("lighting.pad0")
    );
    let p1 = Rect::of(&ui.get("lighting.pad1")).centre();
    ui.press_at(p1, slint::platform::PointerEventButton::Left);
    ui.pump(50);
    assert_eq!(ui.app.lighting.borrow().pads.flashing, vec![1]);
    assert_eq!(g(&ui).get_layer(), "flash-pad");
    ui.release_at(p1, slint::platform::PointerEventButton::Left);
    ui.pump(50);
    assert!(ui.app.lighting.borrow().pads.flashing.is_empty());
    assert_eq!(g(&ui).get_layer(), "latched");
    // A group master.
    ui.click("lighting.master.group1");
    assert!(ui.wait_until(3.0, |u| {
        let l = u
            .app
            .lighting
            .borrow()
            .config
            .settings
            .group_level("Movers");
        l > 0.3 && l < 0.7
    }));
    // Saved with the rig.
    let r = rig(&ui);
    let pads = &r.settings.pads;
    assert_eq!(pads.len(), 3);
    let red = od_lighting::looks::find(pads, 0).unwrap();
    assert_eq!(red.name, "Red wash");
    assert_eq!(red.groups.len(), 1);
    assert_eq!(red.groups["Wash"].color, Some(crate::lighting::SWATCHES[0]));
    assert_eq!(red.groups["Wash"].intensity, Some(0.8));
    assert_eq!(
        od_lighting::looks::find(pads, 1).unwrap().action,
        PadAction::Flash
    );
    let chase = od_lighting::looks::find(pads, 8).unwrap();
    assert_eq!((chase.steps.clone(), chase.step_beats), (vec![0, 1], 2.0));
    assert!(r.settings.group_levels["Movers"] < 0.7);
    // The latch itself is live state, not saved.
    let text = std::fs::read_to_string(crate::lighting::rig_path(&ui.fx.library)).unwrap();
    assert!(!text.contains("latched"));
}

/// Labelled rectangles of the decks, mixer and waveforms.
fn deck_geometry(ui: &Ui) -> Vec<(String, i32, i32, i32, i32)> {
    let mut out = Vec::new();
    for prefix in ["deck1.", "deck2.", "mixer.", "wave.", "club1.", "club2."] {
        for (l, _, e) in ui.with_prefix(prefix) {
            let r = Rect::of(&e);
            out.push((l, r.x as i32, r.y as i32, r.w as i32, r.h as i32));
        }
    }
    out
}

/// [17] The lighting panel (STRIP) takes the browser's place in Perform and
/// Club: at 1440 × 900 and 1280 × 800 every deck, mixer and waveform element
/// keeps its place and size, and what is touched is 44 px or more.
#[test]
fn lighting_panel_keeps_the_decks_and_is_sized_for_touch() {
    for size in [(1440, 900), (1280, 800)] {
        for club in [false, true] {
            let ui = Ui::open(size);
            ui.app.ui.invoke_set_mode("perform".into());
            crate::engine::ensure_engine(&ui.app);
            if club {
                crate::club::set_enabled(&ui.app, true);
            }
            ui.pump(1500);
            let before = deck_geometry(&ui);
            assert!(before.len() > 20, "{size:?} club {club}: {}", before.len());
            ui.click("topbar.toggle.light");
            assert!(ui.wait_until(10.0, |u| u.has("lighting.strip.pad7")));
            ui.pump(1500);
            let after = deck_geometry(&ui);
            assert_eq!(before, after, "{size:?} club {club}: the decks moved");
            for label in [
                "lighting.strip.pad0",
                "lighting.strip.pad7",
                "lighting.strip.page1",
                "lighting.strip.mode.auto",
                "lighting.strip.enable",
                "lighting.strip.hold",
                "lighting.strip.release",
                "lighting.strip.blackout",
                "lighting.strip.flash",
                "lighting.strip.strobe",
                "lighting.strip.fog",
                "lighting.strip.master",
            ] {
                let r = Rect::of(&ui.get(label));
                assert!(r.w >= 44.0 && r.h >= 44.0, "{size:?}: {label} is {r:?}");
                assert!(
                    r.bottom() <= ui.size().1 && r.right() <= ui.size().0,
                    "{size:?}: {label} outside the window"
                );
            }
            // The keys act on the show.
            ui.click("lighting.strip.hold");
            assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().pads.hold));
            ui.click("lighting.strip.release");
            assert!(ui.wait_until(3.0, |u| !u.app.lighting.borrow().pads.hold));
        }
    }
}

/// [18] Palette, the phrase-to-look table, HOLD, a pad over AUTO and
/// RELEASE: the order is shown and followed.
#[test]
fn auto_palette_table_hold_and_release() {
    let ui = Ui::open((1440, 900));
    ui.click("topbar.mode.lighting");
    ui.pump(200);
    patch_two_groups(&ui);
    {
        let mut rt = ui.app.lighting.borrow_mut();
        let mut look = od_lighting::PadLook::new(2, "Drop look");
        look.groups.insert("Wash".into(), Default::default());
        rt.config.settings.pads.push(look);
    }
    ui.click("lighting.space.show");
    ui.click("lighting.mode.auto");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.auto.palette2")));
    ui.click("lighting.auto.palette2");
    assert!(ui.wait_until(
        3.0,
        |u| u.app.lighting.borrow().config.settings.auto.palette == od_lighting::Palette::Cool
    ));
    // OWN: chosen swatches make the palette.
    let own = ui.scroll_into_view("lighting.auto", |u| u.find("lighting.auto.palette6"));
    ui.click_el(&own.expect("OWN in view"));
    ui.pump(100);
    let custom = ui.scroll_into_view("lighting.auto", |u| u.find("lighting.auto.custom4"));
    ui.click_el(&custom.expect("the swatches of OWN"));
    assert!(ui.wait_until(
        3.0,
        |u| u.app.lighting.borrow().config.settings.auto.custom.len() == 1
    ));
    // The order is on screen; AUTO is in control.
    assert!(ui.has("lighting.order"));
    assert_eq!(g(&ui).get_layer(), "base");
    // HOLD.
    ui.click("lighting.auto.hold");
    assert!(ui.wait_until(3.0, |u| g(u).get_layer() == "hold"));
    assert!(g(&ui).get_layer_text().starts_with("AUTO HELD"));
    // A pad latched over AUTO wins; blackout over everything.
    crate::lighting_pads::play(&ui.app, 2, true);
    assert_eq!(g(&ui).get_layer(), "latched");
    assert_eq!(ui.value("lighting.order"), "latched");
    assert!(g(&ui).get_layer_text().contains("DROP LOOK"));
    ui.click("lighting.blackout");
    assert!(ui.wait_until(3.0, |u| g(u).get_layer() == "blackout"));
    ui.click("lighting.blackout");
    // RELEASE hands back to AUTO: the latch and HOLD end.
    ui.click("lighting.auto.release");
    assert!(ui.wait_until(3.0, |u| g(u).get_layer() == "base"));
    let rt = ui.app.lighting.borrow();
    assert!(!rt.pads.hold && rt.pads.latched.is_none());
    drop(rt);
    // DROP (row 3): step back from SWEEP past the programs to the pad.
    let row = PhraseSlot::ALL
        .iter()
        .position(|s| *s == PhraseSlot::Drop)
        .unwrap();
    for _ in 0..5 {
        let label = format!("lighting.auto.row{row}.next");
        let e = ui
            .scroll_into_view("lighting.auto", |u| u.find(&label))
            .expect("row in view");
        ui.click_el(&e);
    }
    assert!(ui.wait_until(
        3.0,
        |u| u.app.lighting.borrow().config.settings.auto.table.drop == AutoPick::Pad(2)
    ));
    assert_eq!(
        ui.value(&format!("lighting.auto.row{row}")),
        "1.3 DROP LOOK"
    );
    let order = ui
        .scroll_into_view("lighting.auto", |u| u.find("lighting.auto.order"))
        .unwrap();
    assert!(
        order.accessible_value().unwrap().contains("HOLD"),
        "the order is written out"
    );
    let r = rig(&ui);
    assert_eq!(r.settings.auto.table.drop, AutoPick::Pad(2));
    assert_eq!(r.settings.auto.palette, od_lighting::Palette::Custom);
}

/// [20] Several universes, the port picker, Art-Net discovery against a
/// fake node on the loopback address, the test pattern and the guided
/// setup. No network, interface or node is needed.
#[test]
fn connecting_interfaces_ports_nodes_test_pattern_and_guide() {
    let ui = Ui::open((1440, 900));
    let link = od_lighting::TestLink::default();
    ui.app.lighting.borrow().use_opener(link.opener());
    let node = od_lighting::FakeNode::start(
        std::net::Ipv4Addr::new(192, 0, 2, 10),
        "Stage left",
        "Test node",
        &[0, 1],
    )
    .unwrap();
    {
        let mut rt = ui.app.lighting.borrow_mut();
        rt.poll_targets = vec![node.addr];
        rt.ports = vec!["/dev/cu.usbserial-DMX1".into()];
        rt.ports_fixed = true;
    }
    ui.click("topbar.mode.lighting");
    ui.pump(200);
    patch_two_groups(&ui);
    ui.click("lighting.setup.output");
    ui.pump(100);
    // Several universes to one Art-Net node: each its own Port-Address.
    let gl = g(&ui);
    gl.invoke_set_output("artnet".into(), "127.0.0.1".into(), "1".into());
    gl.invoke_set_output("artnet".into(), "127.0.0.1".into(), "2".into());
    gl.invoke_set_output("sacn".into(), "".into(), "3".into());
    {
        let rt = ui.app.lighting.borrow();
        let o = &rt.config.outputs;
        assert!(matches!(o[0], (0, OutputKind::ArtNet { universe: 0, .. })));
        assert!(matches!(o[1], (1, OutputKind::ArtNet { universe: 1, .. })));
        assert_eq!(o[2], (2, OutputKind::Sacn { universe: 3 }));
    }
    // Discovery.
    ui.click("lighting.output.artnet");
    ui.click("lighting.output.find");
    assert!(
        ui.wait_until(20.0, |u| u.has("lighting.output.node0")),
        "the fake node answers"
    );
    assert!(ui.value("lighting.output.node0").contains("192.0.2.10"));
    assert!(ui.value("lighting.output.find-text").contains("1"));
    type_into(&ui, "lighting.output.universe", "2");
    ui.click("lighting.output.node0.use");
    assert!(ui.wait_until(3.0, |u| {
        u.app.lighting.borrow().config.outputs.iter().any(|(x, k)| {
            *x == 1
                && *k
                    == OutputKind::ArtNet {
                        target: "192.0.2.10:6454".parse().unwrap(),
                        universe: 1,
                    }
        })
    }));
    // The port picker.
    ui.click("lighting.output.enttec");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.output.port0")));
    ui.click("lighting.output.port0.use");
    assert!(ui.wait_until(3.0, |u| {
        u.app.lighting.borrow().config.outputs.iter().any(|(x, k)| {
            *x == 1 && matches!(k, OutputKind::EnttecPro { device } if device.ends_with("DMX1"))
        })
    }));
    // [21] The output test is one key away in SETUP: the test pattern
    // lights the fixtures one after another.
    ui.click("lighting.test");
    assert!(ui.wait_until(5.0, |u| {
        let rt = u.app.lighting.borrow();
        let s = rt.runner.snapshot.lock().unwrap();
        s.looks.len() == 3 && s.looks.iter().filter(|l| l.intensity > 0.5).count() == 1
    }));
    assert_eq!(g(&ui).get_test(), "each");
    ui.click("lighting.test");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().test.is_none()));
    // The guided setup: Art-Net, the node, universe 2, test, done.
    ui.click("lighting.guide.open");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.guide.artnet")));
    ui.click("lighting.guide.artnet");
    assert!(ui.wait_until(20.0, |u| u.has("lighting.guide.node0")));
    assert!(!ui.has("lighting.guide.apply"));
    ui.click("lighting.guide.node0.use");
    assert!(ui.wait_until(3.0, |u| {
        u.value("lighting.guide.target").contains("192.0.2.10")
    }));
    ui.click("lighting.guide.next");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.guide.universe")));
    // The node's first port (Port-Address 0) suggested universe 1.
    assert!(ui.value("lighting.guide.universe").ends_with('1'));
    ui.click("lighting.guide.universe.up");
    ui.click("lighting.guide.apply");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.guide.works")));
    assert_eq!(
        ui.app.lighting.borrow().test,
        Some(od_lighting::TestPattern::Each)
    );
    ui.click("lighting.guide.fails");
    assert!(ui.wait_until(3.0, |u| u.value("lighting.guide.note").contains("DMX")));
    ui.click("lighting.guide.works");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.guide.done")));
    assert_eq!(ui.app.lighting.borrow().test, None, "test stopped");
    let r = rig(&ui);
    assert!(r.enabled);
    assert!(r.outputs.iter().any(|(x, k)| *x == 1
        && *k
            == OutputKind::ArtNet {
                target: "192.0.2.10:6454".parse().unwrap(),
                universe: 1,
            }));
    ui.click("lighting.guide.done");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.guide.open")));
    drop(node);
}

/// [15] Pads, pages, group masters, HOLD and RELEASE from mapped controls
/// and the deck pads in LIGHTING mode.
#[test]
fn own_pads_from_controllers_and_deck_pads() {
    use od_controller::action::LightingAction as L;
    let ui = Ui::open((1440, 900));
    patch_two_groups(&ui);
    {
        let mut rt = ui.app.lighting.borrow_mut();
        let mut a = od_lighting::PadLook::new(0, "A");
        a.groups.insert("Wash".into(), Default::default());
        let mut b = a.clone();
        b.pad = 9;
        b.name = "B".into();
        b.action = PadAction::Flash;
        rt.config.settings.pads = vec![a, b];
    }
    crate::lighting::controller(
        &ui.app,
        L::Pad {
            pad: 0,
            pressed: true,
        },
    );
    crate::lighting::controller(
        &ui.app,
        L::Pad {
            pad: 0,
            pressed: false,
        },
    );
    assert_eq!(ui.app.lighting.borrow().pads.latched, Some(0));
    crate::lighting::controller(&ui.app, L::PageNext);
    assert_eq!(ui.app.lighting.borrow().page, 1);
    crate::lighting::controller(
        &ui.app,
        L::Pad {
            pad: 1,
            pressed: true,
        },
    );
    assert_eq!(ui.app.lighting.borrow().pads.flashing, vec![9]);
    crate::lighting::controller(
        &ui.app,
        L::Pad {
            pad: 1,
            pressed: false,
        },
    );
    assert!(ui.app.lighting.borrow().pads.flashing.is_empty());
    crate::lighting::controller(
        &ui.app,
        L::GroupMaster {
            group: 0,
            level: 0.25,
        },
    );
    assert_eq!(
        ui.app.lighting.borrow().config.settings.group_level("Wash"),
        0.25
    );
    crate::lighting::controller(&ui.app, L::Hold);
    assert!(ui.app.lighting.borrow().pads.hold);
    crate::lighting::controller(&ui.app, L::Release);
    {
        let rt = ui.app.lighting.borrow();
        assert!(!rt.pads.hold && rt.pads.latched.is_none());
    }
    crate::lighting::controller(&ui.app, L::Page(0));
    // Deck pads in LIGHTING mode: pad A plays the look on 1.1; the empty
    // pad C keeps the built-in look.
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(200);
    ui.app.ui.invoke_deck_pad_mode_set(1, 11);
    ui.pump(100);
    crate::decks::pad(&ui.app, 0, 0, true);
    crate::decks::pad(&ui.app, 0, 0, false);
    assert_eq!(ui.app.lighting.borrow().pads.latched, Some(0));
    crate::decks::pad(&ui.app, 0, 2, true);
    crate::decks::pad(&ui.app, 0, 2, false);
    assert_eq!(
        ui.app.lighting.borrow().config.settings.easy_program,
        od_lighting::Program::Chase
    );
}

