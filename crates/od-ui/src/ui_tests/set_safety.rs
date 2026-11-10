//! Roadmap step 4, "nothing goes wrong during a set": audio health and the
//! gig check, restore after a crash, the performance lock, phase and tempo
//! between decks, and the booth view.

use super::harness::{Fixture, Ui};
use slint::ComponentHandle;

fn perform(ui: &Ui) {
    ui.app.ui.set_mode("perform".into());
    ui.pump(300);
}

fn position(ui: &Ui, deck: usize) -> f64 {
    crate::engine::snapshot(&ui.app).map_or(0.0, |s| s.decks[deck].position)
}

/// Loads fixture track `t` on `deck`, plays it and waits until it runs.
fn play(ui: &Ui, deck: usize, t: usize) {
    crate::decks::load_track(&ui.app, deck, ui.fx.tracks[t].0, None);
    assert!(
        ui.wait_until(30.0, |u| crate::engine::snapshot(&u.app)
            .is_some_and(|s| s.decks[deck].track.is_some())),
        "deck {} loads",
        deck + 1
    );
    crate::engine::deck_cmd(&ui.app, deck, od_engine::DeckCommand::Play);
    assert!(ui.wait_until(10.0, |u| position(u, deck) > 0.3), "deck {} plays", deck + 1);
}

/// [10] The status band always names the output, its latency and the
/// dropouts; losing the device shows the alert at once, the decks hold, and
/// SWITCH OUTPUT brings the sound back on another output with the decks
/// where they were. TRY AGAIN does the same on the lost device.
#[test]
fn device_loss_is_shown_at_once_and_recovers() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    play(&ui, 0, 0);
    let status = ui.value("status.audio");
    assert!(status.starts_with("ok "), "running line: {status}");
    assert!(status.contains("ms"), "with latency: {status}");
    assert!(!ui.has("app.audio-lost"));

    crate::audio_health::inject_loss(&ui.app);
    assert!(
        ui.wait_until(1.0, |u| u.has("app.audio-lost")),
        "the alert shows within a second"
    );
    assert!(ui.value("status.audio").starts_with("lost "));
    let held = position(&ui, 0);
    std::thread::sleep(std::time::Duration::from_millis(150));
    ui.pump(50);
    assert!((position(&ui, 0) - held).abs() < 0.05, "the deck holds while there is no output");

    ui.click("app.audio-lost.switch");
    assert!(ui.wait_until(3.0, |u| !u.has("app.audio-lost")), "the alert goes");
    assert!(ui.value("status.audio").starts_with("ok "));
    assert!(
        ui.wait_until(5.0, |u| position(u, 0) > held + 0.2),
        "deck 1 goes on from {held:.2} s"
    );
    let snap = crate::engine::snapshot(&ui.app).unwrap();
    assert!(snap.decks[0].playing && snap.decks[0].track == Some(ui.fx.tracks[0].0));
    assert!(position(&ui, 0) < held + 3.0, "no jump");

    // Once more, back on the same output.
    crate::audio_health::inject_loss(&ui.app);
    assert!(ui.wait_until(1.0, |u| u.has("app.audio-lost")));
    ui.click("app.audio-lost.retry");
    assert!(ui.wait_until(3.0, |u| !u.has("app.audio-lost")));
    let at = position(&ui, 0);
    assert!(ui.wait_until(5.0, |u| position(u, 0) > at + 0.2), "plays again");
}

/// [10] The gig check names each of outputs, controller, disk space and
/// track files, and a missing file is a struck result.
#[test]
fn gig_check_reports_each_item() {
    let fx = Fixture::new();
    let gone = fx.root.join("music/track3.wav");
    let ui = Ui::start(fx, (1440, 900), false);
    std::fs::remove_file(&gone).unwrap();
    ui.click("topbar.gig.menu");
    ui.pump(100);
    assert!(ui.has("topbar.gig.check"), "the gig menu offers the check");
    // A click outside closes the menu (and is used up by it).
    ui.click_at(
        slint::LogicalPosition::new(700.0, 880.0),
        slint::platform::PointerEventButton::Left,
    );
    ui.pump(100);
    // Popup contents take no synthetic clicks on the testing backend: run
    // the callback the key calls.
    ui.app.ui.global::<crate::GigCheck>().invoke_run();
    assert!(ui.wait_until(5.0, |u| u.has("app.gig-check")), "the band opens");
    let outputs = ui.value("app.gig-check.outputs");
    assert!(outputs.starts_with("warn: ") || outputs.starts_with("ok: "), "{outputs}");
    assert!(outputs.contains("ms"), "names the latency: {outputs}");
    let controller = ui.value("app.gig-check.controller");
    assert!(controller.starts_with("warn: No controller"), "{controller}");
    assert!(ui.value("app.gig-check.disk").contains("GB free"));
    let files = ui.value("app.gig-check.files");
    assert!(files.starts_with("fail: 1 missing: Charlie Tune"), "{files}");
    assert_eq!(ui.value("app.gig-check"), "fail");
    ui.click("app.gig-check.close");
    ui.pump(100);
    assert!(!ui.has("app.gig-check"));
}
