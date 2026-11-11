//! Roadmap 0.9.0, lighting: a separate lighting operator ([34]: a
//! controller given to the lights, OSC in on a port, the music sent to
//! another instance), DJ events as triggers and music sources beyond the
//! decks ([36]), and unattended mode for a fixed venue ([37]), through the
//! real window. OSC goes over the loopback address only; outputs go to a
//! test link.

use std::net::UdpSocket;
use std::time::Duration;

use super::harness::*;
use od_controller::action::LightingAction;
use od_controller::osc::{MusicMessage, OscArg, OscMessage, OscSender};
use od_core::PhraseKind;
use od_lighting::{DjEvent, MusicSource, RigConfig, TriggerTarget};
use slint::ComponentHandle;

fn g(ui: &Ui) -> crate::Lighting<'_> {
    ui.app.ui.global::<crate::Lighting>()
}

fn rig(ui: &Ui) -> RigConfig {
    RigConfig::load(&crate::lighting::rig_path(&ui.fx.library)).unwrap()
}

fn lighting(ui: &Ui) {
    ui.click("topbar.mode.lighting");
    ui.pump(200);
}

/// Two pars and two looks on pads 1.1 and 1.2.
fn patch_and_looks(ui: &Ui) {
    g(ui).invoke_add_fixtures(2, "Par".into(), "Wash".into(), "2".into(), "".into(), "".into());
    assert!(ui.wait_until(5.0, |u| u.app.lighting.borrow().config.patch.fixtures.len() == 2));
    crate::lighting::edit(&ui.app, |rt| {
        for (pad, name, color) in [(0u16, "Red wash", [1.0, 0.0, 0.0]), (1, "Blue wash", [0.0, 0.0, 1.0])] {
            let mut l = od_lighting::PadLook::new(pad, name);
            l.groups.insert(
                "Wash".into(),
                od_lighting::GroupSettings {
                    program: Some(od_lighting::Program::Static),
                    color: Some(color),
                    intensity: Some(1.0),
                    ..Default::default()
                },
            );
            rt.config.settings.pads.push(l);
        }
    });
}

/// OSC in on a free port of this computer (the port field takes 1024 and
/// up; a test asks for any free one).
fn osc_on(ui: &Ui) -> OscSender {
    ui.app.lighting.borrow_mut().config.remote.osc_port = 0;
    ui.click("lighting.space.setup");
    // The REMOTE key toggles; open it for sure.
    g(ui).set_setup_tab("remote".into());
    ui.pump(50);
    ui.click("lighting.remote.osc-in");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().remote.osc.is_some()), "listening");
    let port = ui.app.lighting.borrow().remote.osc.as_ref().unwrap().local_addr().port();
    OscSender::new(([127, 0, 0, 1], port).into()).unwrap()
}

fn send(tx: &OscSender, addr: &str, args: Vec<OscArg>) {
    tx.send(&OscMessage::new(addr, args)).unwrap();
}

/// [34] A second controller is given to the lights (and given back);
/// OSC in plays pads, blackout and the master from a loopback sender; the
/// music goes out to another instance over OSC.
#[test]
fn a_lighting_controller_osc_in_and_the_music_sent_out() {
    let ui = Ui::open((1440, 900));
    lighting(&ui);
    patch_and_looks(&ui);
    // The controllers to choose from (no MIDI in a test).
    {
        let mut rt = ui.app.lighting.borrow_mut();
        rt.remote.midi_inputs = vec!["DJ Controller MIDI 1".into(), "Pad Box MIDI 1".into()];
        rt.remote.midi_fixed = true;
    }
    crate::lighting::refresh(&ui.app);
    ui.click("lighting.space.setup");
    ui.click("lighting.setup.remote");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.remote.controller1.use")));
    ui.click("lighting.remote.controller1.use");
    assert!(ui.wait_until(3.0, |u| rig(u).remote.controller == "Pad Box MIDI 1"));
    assert_eq!(g(&ui).get_light_controller(), "Pad Box MIDI 1");
    assert!(ui.value("lighting.remote.controller-note").contains("NOT CONNECTED"));
    // The assignment reaches the controller layer (the manager is not
    // running headless; the port is remembered for when it starts).
    ui.click("lighting.remote.controller1.use");
    assert!(ui.wait_until(3.0, |u| rig(u).remote.controller.is_empty()), "given back");

    // OSC in, off by default.
    assert!(!rig(&ui).remote.osc_in);
    let tx = osc_on(&ui);
    assert!(rig(&ui).remote.osc_in);
    send(&tx, "/opendjdecks/light/pad", vec![OscArg::Int(2)]);
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().pads.latched == Some(1)), "pad 1.2 latched over OSC");
    send(&tx, "/opendjdecks/light/master", vec![OscArg::Float(0.5)]);
    send(&tx, "/opendjdecks/light/blackout", vec![OscArg::Int(1)]);
    assert!(ui.wait_until(3.0, |u| {
        let s = &u.app.lighting.borrow().config.settings;
        s.blackout && s.master == 0.5
    }));
    send(&tx, "/opendjdecks/light/blackout", vec![OscArg::Int(0)]);
    send(&tx, "/somebody/else", vec![]);
    assert!(ui.wait_until(3.0, |u| !u.app.lighting.borrow().config.settings.blackout));
    assert!(ui.wait_until(3.0, |u| u.value("lighting.remote.osc-note").contains("· 4 MESSAGES")), "{}", ui.value("lighting.remote.osc-note"));

    // The music sent to another instance: a socket on this computer
    // stands for it.
    let other = UdpSocket::bind("127.0.0.1:0").unwrap();
    other.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let to = other.local_addr().unwrap().to_string();
    {
        let send = ui.get("lighting.remote.send-to");
        ui.click_el(&send);
        ui.type_text(&to);
    }
    ui.click("lighting.remote.send");
    assert!(ui.wait_until(3.0, |u| rig(u).remote.send && rig(u).remote.send_to == to), "{:?}", rig(&ui).remote);
    let mut buf = [0u8; 512];
    let n = other.recv(&mut buf).expect("music arrives");
    let msgs = od_controller::osc::decode(&buf[..n]).unwrap();
    let m = MusicMessage::from_osc(&msgs[0]).expect("a music message");
    assert!(!m.playing && m.bpm > 0.0, "nothing plays here: {m:?}");
    ui.click("lighting.remote.send");
    assert!(ui.wait_until(3.0, |u| !rig(u).remote.send));
    // Off again: the port is closed.
    ui.click("lighting.remote.osc-in");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().remote.osc.is_none()));
}

/// [36] DJ events tied to pads and cues in TRIGGERS; the show follows the
/// network (a loopback sender stands for the other instance): a drop
/// plays its pad, a breakdown releases; the audio input has no device in
/// a test and says so.
#[test]
fn dj_events_trigger_pads_and_the_show_follows_the_network() {
    let ui = Ui::open((1440, 900));
    lighting(&ui);
    patch_and_looks(&ui);
    let tx = osc_on(&ui);
    ui.click("lighting.space.show");
    ui.click("lighting.centre.triggers");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.triggers.row7")), "a row per event");
    let drop = DjEvent::ALL.iter().position(|e| *e == DjEvent::Drop).unwrap();
    let down = DjEvent::ALL.iter().position(|e| *e == DjEvent::Breakdown).unwrap();
    // DROP: nothing → RELEASE → pad 1.1 → pad 1.2 (no cue stack here).
    for _ in 0..3 {
        ui.click(&format!("lighting.triggers.row{drop}.plays.next"));
    }
    for _ in 0..3 {
        ui.click(&format!("lighting.triggers.row{drop}.for.up"));
    }
    ui.click(&format!("lighting.triggers.row{down}.plays.next"));
    assert!(ui.wait_until(3.0, |u| {
        let t = &u.app.lighting.borrow().config.settings.triggers;
        t.len() == 2
            && t.iter().any(|t| t.event == DjEvent::Drop && t.target == TriggerTarget::Pad(1) && t.beats == 16.0)
            && t.iter().any(|t| t.event == DjEvent::Breakdown && t.target == TriggerTarget::Release)
    }), "{:?}", ui.app.lighting.borrow().config.settings.triggers);
    assert!(ui.value(&format!("lighting.triggers.row{drop}")).contains("BLUE WASH"));
    assert_eq!(rig(&ui).settings.triggers.len(), 2, "saved with the show");

    // The audio input: no device in a test.
    ui.click("lighting.source.input");
    assert!(ui.wait_until(3.0, |u| u.value("lighting.source.text").contains("NO AUDIO INPUT")));
    assert_eq!(rig(&ui).music, MusicSource::Input);
    // The network.
    ui.click("lighting.source.network");
    assert!(ui.wait_until(3.0, |u| rig(u).music == MusicSource::Network));
    let music = |beat: f64, phrase| MusicMessage {
        bpm: 120.0,
        beat,
        playing: true,
        energy: 0.8,
        phrase: Some(phrase),
        hue: 0.5,
        remaining: None,
    };
    // A verse, then the drop: pad 1.2 plays for 16 beats.
    let mut beat = 0.0;
    let mut feed = |phrase, secs: f32| {
        for _ in 0..(secs * 20.0) as usize {
            tx.send(&music(beat, phrase).to_osc()).unwrap();
            beat += 0.1;
            ui.pump(50);
            std::thread::sleep(Duration::from_millis(50));
        }
    };
    feed(PhraseKind::Verse, 1.0);
    assert!(ui.wait_until(3.0, |u| u.value("lighting.source.text").contains("120 BPM")), "{}", ui.value("lighting.source.text"));
    feed(PhraseKind::Chorus, 0.5);
    assert!(ui.wait_until(3.0, |u| {
        u.app.lighting.borrow().runner.snapshot.lock().unwrap().trigger.is_some_and(|t| t.target == TriggerTarget::Pad(1))
    }), "the drop plays its pad");
    assert!(ui.wait_until(3.0, |u| u.value(&format!("lighting.triggers.row{drop}")).ends_with("active")));
    assert!(ui.value("lighting.triggers.last").contains("DROP"));
    let blue = ui.app.lighting.borrow().runner.snapshot.lock().unwrap().looks[0].color;
    assert!(blue[2] > 0.5 && blue[0] < 0.5, "blue wash: {blue:?}");
    // The breakdown releases it.
    feed(PhraseKind::Down, 0.5);
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().runner.snapshot.lock().unwrap().trigger.is_none()));
    // Nothing arrives: the network goes quiet after two seconds.
    assert!(ui.wait_until(5.0, |u| u.value("lighting.source.text").contains("NOTHING ARRIVES")));
}

/// [37] Unattended mode needs the PIN; locked, the show controls, a
/// controller and OSC do nothing (BLACKOUT stays); the PIN takes over.
/// After a restart the show starts by itself, still locked.
#[test]
fn unattended_mode_is_locked_and_starts_by_itself() {
    let ui = Ui::open((1440, 900));
    lighting(&ui);
    patch_and_looks(&ui);
    let tx = osc_on(&ui);
    ui.click("lighting.setup.safety");
    // Without a PIN it cannot be turned on.
    g(&ui).invoke_set_unattended(true);
    ui.pump(50);
    assert!(!ui.app.lighting.borrow().config.unattended.enabled);
    {
        ui.click("lighting.pin.new");
        ui.type_text("2468");
        ui.click("lighting.pin.set");
    }
    assert!(ui.wait_until(3.0, |u| !u.app.lighting.borrow().config.safety.pin.is_empty()));
    let on = ui.scroll_into_view("lighting.safety", |u| u.find("lighting.unattended"));
    ui.click_el(&on.expect("RUN UNATTENDED"));
    assert!(ui.wait_until(3.0, |u| rig(u).unattended.enabled));
    let calm = ui.scroll_into_view("lighting.safety", |u| u.find("lighting.unattended.calm.down"));
    ui.click_el(&calm.expect("CALM AFTER"));
    assert!(ui.wait_until(3.0, |u| rig(u).unattended.calm_after == 10.0));
    // Not locked yet: the show is free while the setup is open.
    assert!(!g(&ui).get_show_locked());
    ui.click("lighting.pin.lock");
    assert!(ui.wait_until(3.0, |u| u.app.ui.global::<crate::Lighting>().get_show_locked()));
    ui.click("lighting.space.show");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.unattended.state")));
    assert!(!ui.has("lighting.mode.auto") && !ui.has("lighting.flash"), "the show controls are locked");
    assert!(ui.has("lighting.blackout"), "BLACKOUT stays");
    // A controller, OSC and the deck pads do nothing.
    crate::lighting::controller(&ui.app, LightingAction::Flash(true));
    crate::lighting::pad(&ui.app, 0, true);
    send(&tx, "/opendjdecks/light/pad", vec![OscArg::Int(1)]);
    send(&tx, "/opendjdecks/light/blackout", vec![OscArg::Int(1)]);
    ui.pump(300);
    {
        let rt = ui.app.lighting.borrow();
        assert!(!rt.momentary.flash && rt.pads.latched.is_none() && !rt.config.settings.blackout, "ignored");
    }
    // The Perform strip is locked too.
    g(&ui).set_panel(true);
    ui.click("topbar.mode.perform");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.strip.unattended")));
    assert!(!ui.has("lighting.strip.flash") && ui.has("lighting.strip.blackout"));
    g(&ui).set_panel(false);
    lighting(&ui);
    // The PIN takes over.
    ui.click("lighting.unattended.pin");
    ui.type_text("1111");
    ui.click("lighting.unattended.unlock");
    assert!(ui.wait_until(3.0, |u| !u.app.ui.global::<crate::Lighting>().get_pin_error().is_empty()), "wrong PIN");
    ui.click("lighting.unattended.pin");
    ui.type_text("2468");
    ui.click("lighting.unattended.unlock");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.mode.auto")), "taken over");
    crate::lighting::controller(&ui.app, LightingAction::Flash(true));
    assert!(ui.app.lighting.borrow().momentary.flash, "the keys work again");
    crate::lighting::controller(&ui.app, LightingAction::Flash(false));
    // Clearing the PIN ends unattended mode.
    ui.click("lighting.space.setup");
    ui.click("lighting.setup.safety");
    ui.click("lighting.pin.set");
    assert!(ui.wait_until(3.0, |u| !rig(u).unattended.enabled && rig(u).safety.pin.is_empty()));
}

/// [37] A venue that runs unattended with autostart: on the next start
/// the app opens LIGHTING in SHOW with DMX OUT on, locked.
#[test]
fn an_unattended_show_starts_with_the_app() {
    let fx = Fixture::new();
    let mut cfg = RigConfig::default();
    od_lighting::add_fixtures(&mut cfg.patch, 2, 2, Some(0), None, "Par", "Wash").unwrap();
    cfg.safety.pin = "2468".into();
    cfg.unattended = od_lighting::Unattended {
        enabled: true,
        autostart: true,
        calm_after: 10.0,
    };
    // No output: a test sends nothing anywhere.
    cfg.enabled = false;
    cfg.save(&crate::lighting::rig_path(&fx.library)).unwrap();
    let ui = Ui::start(fx, (1440, 900), false);
    assert_eq!(ui.app.ui.get_mode(), "lighting");
    assert_eq!(g(&ui).get_space(), "show");
    assert!(ui.app.lighting.borrow().config.enabled, "DMX OUT on");
    assert!(g(&ui).get_show_locked(), "locked after the restart");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.unattended.state")));
    // Silence: after the time set, the calm look.
    assert!(ui.wait_until(20.0, |u| {
        u.app.lighting.borrow().runner.snapshot.lock().unwrap().calm >= 1.0
    }), "the calm look on silence");
    assert!(ui.wait_until(3.0, |u| u.value("lighting.unattended.state").contains("CALM")), "{}", ui.value("lighting.unattended.state"));
}
