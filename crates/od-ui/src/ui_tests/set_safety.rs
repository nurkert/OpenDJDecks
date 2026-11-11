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

const CRASH_ROOT: &str = "OD_UI_CRASH_CHILD_ROOT";

/// The child of `killed_mid_set_restores_decks_and_keeps_the_recording`:
/// a set in progress (deck 1 playing, deck 2 cued with its tempo moved, a
/// recording running) until it is killed.
#[test]
#[ignore = "run by killed_mid_set_restores_decks_and_keeps_the_recording in a child process"]
fn crash_child_plays_and_records() {
    let Some(root) = std::env::var_os(CRASH_ROOT) else { return };
    let ui = Ui::start(Fixture::existing(root.into()), (1440, 900), false);
    perform(&ui);
    crate::decks::load_track(&ui.app, 1, ui.fx.tracks[1].0, None);
    play(&ui, 0, 0);
    crate::engine::deck_cmd(&ui.app, 1, od_engine::DeckCommand::SetTempoFader(0.3));
    crate::engine::deck_cmd(&ui.app, 1, od_engine::DeckCommand::Seek { seconds: 2.0 });
    crate::recorder::toggle_from_controller(&ui.app);
    let session = crate::restore::session_path(&ui.fx.library);
    let started = std::time::Instant::now();
    let mut ready = false;
    loop {
        ui.pump(16);
        std::thread::sleep(std::time::Duration::from_millis(8));
        if !ready
            && crate::restore::read(&session).is_some_and(|s| {
                s.decks.len() == 2
                    && s.decks[0].position > 2.5
                    && s.recording.is_some()
                    && (s.decks[1].tempo_fader - 0.3).abs() < 1e-3
            })
        {
            println!("READY");
            ready = true;
        }
        if started.elapsed() > std::time::Duration::from_secs(120) {
            std::process::exit(2);
        }
    }
}

/// Kills the child if the test fails before it does.
struct Child(std::process::Child);

impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// [9] A set killed with SIGKILL comes back: the next start offers the
/// decks, keeps the recording (repaired, listed, with its tracklist) and
/// RESTORE DECKS loads the tracks at their positions with their tempo,
/// waiting for PLAY.
#[test]
fn killed_mid_set_restores_decks_and_keeps_the_recording() {
    use std::io::BufRead;
    let fx = Fixture::new();
    let mut child = Child(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "ui_tests::set_safety::crash_child_plays_and_records",
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CRASH_ROOT, &fx.root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("child starts"),
    );
    let out = std::io::BufReader::new(child.0.stdout.take().unwrap());
    let ready = out.lines().map_while(Result::ok).any(|l| l.contains("READY"));
    assert!(ready, "the child set got going");
    std::thread::sleep(std::time::Duration::from_millis(300));
    // SIGKILL on Unix: no destructor, no clean quit.
    child.0.kill().unwrap();
    child.0.wait().unwrap();

    let session = crate::restore::session_path(&fx.library);
    let saved = crate::restore::read(&session).expect("the state is on the disk");
    assert!(!saved.clean, "a killed run is not clean");
    let rec = saved.recording.clone().expect("the recording is named");
    let at = saved.decks[0].position;

    let ids = (fx.tracks[0].0, fx.tracks[1].0);
    let ui = Ui::start(fx, (1440, 900), false);
    assert!(ui.has("app.restore"), "the restore band is offered");
    let offer = ui.value("app.restore");
    assert!(offer.contains("Deck 1: Alpha Tune at 0:0"), "{offer}");
    assert!(offer.contains("Deck 2: Bravo Tune at 0:02"), "{offer}");
    assert!(offer.contains("was kept"), "{offer}");
    // The recording plays and is listed.
    let r = hound::WavReader::open(&rec.path).expect("the recording is a valid WAV");
    let secs = r.duration() as f64 / r.spec().sample_rate as f64;
    assert!(secs > 0.5, "{secs:.2} s kept");
    let listed: i64 = ui
        .app
        .state
        .borrow()
        .lib
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM recording WHERE path = ?1",
            [rec.path.display().to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(listed, 1);

    ui.click("app.restore.restore");
    assert!(
        ui.wait_until(30.0, |u| crate::engine::snapshot(&u.app).is_some_and(|s| {
            s.decks[0].track == Some(ids.0)
                && s.decks[1].track == Some(ids.1)
                && (s.decks[0].position - at).abs() < 0.05
                && (s.decks[1].tempo_fader - 0.3).abs() < 1e-3
        })),
        "tracks, positions and tempo are back"
    );
    let s = crate::engine::snapshot(&ui.app).unwrap();
    assert!(!s.decks[0].playing && !s.decks[1].playing, "the decks wait for PLAY");
    assert!((s.decks[1].position - 2.0).abs() < 0.3, "deck 2 at {}", s.decks[1].position);
    assert!(!ui.has("app.restore"), "the offer goes");
    assert_eq!(ui.app.ui.get_mode(), "perform");
}

fn status(ui: &Ui) -> String {
    ui.app.ui.get_status_text().to_string()
}

/// [11] Locked, nothing happens by accident: no load onto the playing deck
/// (a stopped deck still loads), no deleting a list or a hot cue, no grid
/// edit; each says why. While a deck plays the computer is kept awake and
/// background work waits, and it resumes after the decks stop.
#[test]
fn lock_blocks_accidents_and_playing_keeps_awake() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    play(&ui, 0, 0);
    assert!(ui.wait_until(2.0, |_| crate::perform_lock::paused()), "background work waits");
    assert!(ui.app.analysis.is_paused());
    #[cfg(target_os = "macos")]
    assert!(crate::perform_lock::awake(), "the Mac is kept awake");
    assert_eq!(ui.value("topbar.lock"), "playing");

    ui.click("topbar.lock");
    assert!(crate::perform_lock::locked());
    assert!(ui.app.ui.global::<crate::PerformLock>().get_locked());

    // Loading onto the playing deck.
    crate::decks::load_track(&ui.app, 0, ui.fx.tracks[1].0, None);
    ui.pump(200);
    assert!(status(&ui).starts_with("Locked: deck 1 is playing"), "{}", status(&ui));
    assert_eq!(
        crate::engine::snapshot(&ui.app).unwrap().decks[0].track,
        Some(ui.fx.tracks[0].0),
        "deck 1 keeps its track"
    );
    // A stopped deck still loads.
    crate::decks::load_track(&ui.app, 1, ui.fx.tracks[1].0, None);
    assert!(ui.wait_until(20.0, |u| crate::engine::snapshot(&u.app)
        .is_some_and(|s| s.decks[1].track == Some(u.fx.tracks[1].0))));

    // Deleting a list.
    ui.app.state.borrow_mut().browser.node = Some(ui.fx.playlist);
    ui.app.ui.invoke_delete_node();
    assert!(ui.fx.lib().node(ui.fx.playlist).is_ok(), "the list stays");
    assert_eq!(status(&ui), "Locked: nothing is deleted until you unlock");
    // Deleting a hot cue.
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::SetHotCue { slot: 0 });
    assert!(
        ui.wait_until(3.0, |u| crate::engine::snapshot(&u.app).unwrap().decks[0].hot_cues[0].is_some()),
        "hot cue A set"
    );
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::DeleteHotCue { slot: 0 });
    ui.pump(100);
    assert!(crate::engine::snapshot(&ui.app).unwrap().decks[0].hot_cues[0].is_some(), "hot cue A stays");
    // Moving the grid.
    let grid = ui.fx.lib().beatgrid(ui.fx.tracks[0].0).unwrap();
    crate::deck_grid::act(&ui.app, 0, "shift", "10");
    ui.pump(100);
    assert_eq!(ui.fx.lib().beatgrid(ui.fx.tracks[0].0).unwrap(), grid, "the grid stays");
    assert_eq!(status(&ui), "Locked: the beatgrid stays as it is until you unlock");

    // Unlocked, the same load goes through.
    ui.click("topbar.lock");
    assert!(!crate::perform_lock::locked());
    crate::decks::load_track(&ui.app, 0, ui.fx.tracks[2].0, None);
    assert!(ui.wait_until(20.0, |u| crate::engine::snapshot(&u.app)
        .is_some_and(|s| s.decks[0].track == Some(u.fx.tracks[2].0))));

    // Decks stopped: background work resumes and the Mac may sleep again.
    for d in 0..2 {
        crate::engine::deck_cmd(&ui.app, d, od_engine::DeckCommand::Pause);
    }
    assert!(
        ui.wait_until(10.0, |_| !crate::perform_lock::paused()),
        "background work resumes after the decks stop"
    );
    assert!(!ui.app.analysis.is_paused());
    assert!(!crate::perform_lock::awake());
    assert_eq!(ui.value("topbar.lock"), "idle");
}

fn phase_rows(ui: &Ui) -> Vec<crate::PhaseRow> {
    use slint::Model;
    ui.app.ui.global::<crate::PhaseMeter>().get_rows().iter().collect()
}

/// [12] Beatmatching can be judged from the screen: the strip under the
/// waveforms names the tempo difference and where the beat falls, and says
/// IN SYNC only once both match. PRACTICE turns SYNC off and keeps it off.
#[test]
fn phase_meter_shows_tempo_and_phase_and_practice_keeps_sync_off() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    assert!(ui.has("wave.phase"), "the strip is in Perform");
    play(&ui, 0, 0);
    play(&ui, 1, 1);
    assert!(ui.wait_until(3.0, |u| phase_rows(u).len() == 1), "one deck against the other");
    let reference = ui.app.ui.global::<crate::PhaseMeter>().get_reference();
    let other = if reference == 1 { 1 } else { 0 };
    assert_eq!(phase_rows(&ui)[0].deck as usize, other + 1);

    // A different tempo: named, not in sync.
    crate::engine::deck_cmd(&ui.app, other, od_engine::DeckCommand::SetTempoFader(0.5));
    assert!(ui.wait_until(3.0, |u| phase_rows(u)[0].tempo.starts_with('+')));
    let row = phase_rows(&ui)[0].clone();
    assert!(row.tempo.ends_with("BPM") && !row.in_sync, "{row:?}");
    let label = format!("wave.phase.deck{}", other + 1);
    assert!(ui.value(&label).contains("BPM"));

    // SYNC matches tempo and phase: IN SYNC.
    crate::engine::deck_cmd(&ui.app, other, od_engine::DeckCommand::SetSync(true));
    assert!(
        ui.wait_until(5.0, |u| phase_rows(u).first().is_some_and(|r| r.in_sync)),
        "in sync once tempo and phase match: {:?}",
        phase_rows(&ui)
    );
    assert!(ui.value(&label).starts_with("in sync"));
    assert_eq!(phase_rows(&ui)[0].tempo, "±0.00 BPM");

    // Practice: SYNC goes off and stays off.
    ui.click("wave.phase.practice");
    assert!(crate::phase::practice());
    assert!(ui.wait_until(2.0, |u| crate::engine::snapshot(&u.app)
        .is_some_and(|s| !s.decks[0].sync && !s.decks[1].sync)));
    let sync_key = format!("deck{}.beat-sync", other + 1);
    ui.click(&sync_key);
    crate::engine::deck_cmd(&ui.app, other, od_engine::DeckCommand::SetSync(true));
    ui.pump(200);
    assert!(!crate::engine::snapshot(&ui.app).unwrap().decks[other].sync, "SYNC stays off");
    assert!(status(&ui).starts_with("Practice: SYNC stays off"), "{}", status(&ui));
    ui.click("wave.phase.practice");
    assert!(!crate::phase::practice());
}


/// [58] The booth view opens in its own window and shows the decks of
/// Perform as they play: BPM, key, remaining time and phrase, at a size
/// that grows with the window (hero figures of about 96 px on a 1080p
/// screen, three times the deck's).
#[test]
fn booth_view_shows_the_decks_large() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    play(&ui, 0, 0);
    crate::commands::run(&ui.app, "view.booth");
    ui.pump(100);
    let w = ui
        .app
        .booth_window
        .borrow()
        .as_ref()
        .map(|w| w.clone_strong())
        .expect("booth window");
    w.window().set_size(slint::LogicalSize::new(1920.0, 1080.0));
    ui.pump(300);
    let find = |label: &str| super::harness::find_all(&w, label).next();
    let value = |label: &str| {
        find(label)
            .and_then(|e| e.accessible_value())
            .map(|v| v.to_string())
            .unwrap_or_default()
    };
    assert!(find("booth.deck1").is_some() && find("booth.deck2").is_some(), "both decks");
    assert!(ui.wait_until(3.0, |_| !value("booth.deck1.bpm").is_empty()));
    let deck = super::flows::deck_views(&ui)[0].clone();
    assert_eq!(value("booth.deck1.bpm"), deck.bpm.as_str());
    assert_eq!(value("booth.deck1.key"), deck.key.as_str());
    // The remaining time runs with the deck.
    let before = value("booth.deck1.remaining");
    assert!(ui.wait_until(3.0, |_| value("booth.deck1.remaining") != before), "remaining time runs");
    assert!(find("booth.deck1.phrase").is_some());
    // Large: the BPM reading (caption and value) at 1080p.
    let bpm = super::harness::Rect::of(&find("booth.deck1.bpm").unwrap());
    assert!(bpm.h >= 120.0, "BPM reading {} px tall", bpm.h);
    let small = super::harness::Rect::of(&ui.get("deck1.bpm"));
    assert!(bpm.h >= 2.5 * small.h, "{} vs the deck's {}", bpm.h, small.h);
    // Double-click: full screen.
    w.global::<crate::Booth>().invoke_toggle_full_screen();
    ui.pump(50);
}
