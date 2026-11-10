//! For the mix: the mixer for bass swaps ([55]) and the library import you
//! can check ([54]), driven through the real window.

use super::harness::*;
use od_controller::{ControlAction, EqBand, MixerAction};

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
}

fn snapshot(ui: &Ui) -> od_engine::EngineState {
    crate::engine::snapshot(&ui.app).expect("engine runs")
}

fn kills(ui: &Ui, ch: usize) -> [bool; 3] {
    snapshot(ui).channels[ch].eq_kill
}

/// Loads fixture track `t` on `deck` and waits for it.
fn load(ui: &Ui, deck: usize, t: usize) {
    crate::decks::load_track(&ui.app, deck, ui.fx.tracks[t].0, None);
    assert!(
        ui.wait_until(30.0, |u| crate::engine::snapshot(&u.app)
            .is_some_and(|s| s.decks[deck].track.is_some())),
        "deck {} loads",
        deck + 1
    );
}

/// Where deck 1 is in its bar, in beats from the downbeat (0..4).
fn bar_phase(ui: &Ui) -> f64 {
    let d = snapshot(ui).decks[0];
    f64::from(d.beat_in_bar.max(1) - 1) + d.beat.rem_euclid(1.0)
}

/// Waits until deck 1 (the master) is early in its bar, so a swap on the
/// next bar has more than two beats to wait.
fn early_in_bar(ui: &Ui) {
    assert!(ui.wait_until(5.0, |u| {
        (0.2..1.5).contains(&bar_phase(u))
    }));
}

/// [55] A kill per band, values under the knobs, double-click reset, the
/// one-tap swap now and on the next bar (a second tap cancels), the clip
/// lamp, and kill / swap from a controller mapping.
#[test]
fn mixer_kills_swaps_values_and_clip() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    load(&ui, 0, 0);
    load(&ui, 1, 1);
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Play);
    assert!(ui.wait_until(10.0, |u| snapshot(u).decks[0].playing));
    assert!(ui.wait_until(5.0, |u| u.has("mixer.ch2.eq-low.kill")));
    for ch in 1..=2 {
        for (band, name) in [("eq-high", "HI"), ("eq-mid", "MID"), ("eq-low", "LOW")] {
            assert!(ui.has(&format!("mixer.ch{ch}.{band}.kill")), "ch{ch} {band} kill");
            // At the centre the legend names the band; turned, it shows the value.
            assert_eq!(ui.value(&format!("mixer.ch{ch}.{band}.value")), name);
        }
        assert!(ui.has(&format!("fx.channel-fx.ch{ch}.value")));
        assert!(ui.has(&format!("mixer.ch{ch}.clip")));
    }

    // Kill: the band goes, the knob stays.
    ui.click("mixer.ch2.eq-low.kill");
    assert!(ui.wait_until(5.0, |u| kills(u, 1) == [true, false, false]));
    assert!(ui.wait_until(2.0, |u| u.value("mixer.ch2.eq-low.value") == "KILL"));
    assert_eq!(snapshot(&ui).channels[1].eq[0], 0.0);

    // Values and double-click reset.
    // The legend names the band at rest; the value shows while the knob
    // is turned (drag readout) and in the knob's arc.
    ui.app.ui.invoke_mixer_eq(1, 2, -0.5);
    assert!(ui.wait_until(5.0, |u| (snapshot(u).channels[0].eq[2] + 0.5).abs() < 1e-3));
    assert_eq!(ui.value("mixer.ch1.eq-high.value"), "HI");
    ui.double_click("mixer.ch1.eq-high");
    assert!(ui.wait_until(5.0, |u| snapshot(u).channels[0].eq[2] == 0.0));
    assert!(ui.wait_until(2.0, |u| u.value("mixer.ch1.eq-high.value") == "HI"));
    ui.app.ui.invoke_mixer_channel_fx(2, -0.4);
    assert!(ui.wait_until(5.0, |u| (snapshot(u).channels[1].channel_fx_amount + 0.4).abs() < 1e-3));
    ui.double_click("fx.channel-fx.ch2");
    assert!(ui.wait_until(5.0, |u| snapshot(u).channels[1].channel_fx_amount == 0.0));

    // Swap now: the kill moves to deck 1 at once.
    assert_eq!(ui.value("mixer.swap.at"), "NOW");
    ui.click("mixer.swap.low");
    assert!(ui.wait_until(5.0, |u| kills(u, 0)[0] && !kills(u, 1)[0]));

    // Swap on the next bar: it waits (the key is armed), then lands.
    ui.click("mixer.swap.at");
    ui.click("mixer.swap.at");
    assert!(ui.wait_until(2.0, |u| u.value("mixer.swap.at") == "BAR"));
    early_in_bar(&ui);
    ui.click("mixer.swap.low");
    assert!(ui.wait_until(1.0, |u| u.value("mixer.swap.low") == "waiting"));
    assert!(kills(&ui, 0)[0], "nothing moves before the bar");
    assert!(ui.wait_until(6.0, |u| kills(u, 1)[0] && !kills(u, 0)[0]), "lands on the bar");
    let phase = bar_phase(&ui);
    assert!(phase < 0.5 || phase > 3.9, "landed at bar phase {phase}");
    assert!(ui.wait_until(2.0, |u| u.value("mixer.swap.low").is_empty()));

    // A second tap cancels a waiting swap.
    early_in_bar(&ui);
    ui.click("mixer.swap.low");
    assert!(ui.wait_until(1.0, |u| u.value("mixer.swap.low") == "waiting"));
    ui.click("mixer.swap.low");
    assert!(ui.wait_until(1.0, |u| u.value("mixer.swap.low").is_empty()));
    ui.pump(2500);
    assert!(kills(&ui, 1)[0] && !kills(&ui, 0)[0], "cancelled: no swap");

    // Controller mappings: kill MID on channel 1, swap HI.
    crate::controller::handle_action(
        &ui.app,
        ControlAction::Mixer {
            channel: 1,
            action: MixerAction::EqKill(EqBand::Mid),
        },
    );
    assert!(ui.wait_until(5.0, |u| kills(u, 0)[1]));
    ui.click("mixer.swap.at"); // BAR → NOW
    crate::controller::handle_action(&ui.app, ControlAction::EqSwap(EqBand::Mid));
    assert!(ui.wait_until(5.0, |u| kills(u, 1)[1] && !kills(u, 0)[1]));

    // Clip: a hot trim lights the lamp; a click clears it.
    ui.app.ui.invoke_mixer_trim(1, 1.0);
    assert!(
        ui.wait_until(10.0, |u| u.value("mixer.ch1.clip") == "clip"),
        "the lamp lights on clipping"
    );
    ui.app.ui.invoke_mixer_trim(1, 0.0);
    ui.pump(200);
    // Samples already on their way may still clip: clear once the level is
    // down (on a loaded machine that takes a few blocks).
    assert!(ui.wait_until(10.0, |u| snapshot(u).channels[0].meter.peak < 0.95));
    ui.click("mixer.ch1.clip");
    assert!(ui.wait_until(5.0, |u| u.value("mixer.ch1.clip").is_empty()));
}

/// [55] The larger knobs line up in one column per strip, with the kill
/// beside each EQ knob; the strips keep their 50 px and the swap row fits
/// over them, so the decks keep their width (desktop, laptop, four decks).
#[test]
fn mixer_rows_line_up() {
    for size in [(1440, 900), (1280, 800)] {
        let ui = Ui::open(size);
        perform(&ui);
        assert!(ui.wait_until(5.0, |u| u.has("mixer.ch1.eq-low.kill")));
        for ch in 1..=2 {
            let x = |l: &str| Rect::of(&ui.get(&format!("mixer.ch{ch}.{l}"))).centre().x;
            let knob = x("eq-high");
            for band in ["eq-mid", "eq-low"] {
                assert!((x(band) - knob).abs() <= 1.0, "{size:?} ch{ch} {band}");
                // The legend under the knob is the kill key.
                let kill = Rect::of(&ui.get(&format!("mixer.ch{ch}.{band}.kill")));
                let k = Rect::of(&ui.get(&format!("mixer.ch{ch}.{band}")));
                assert!(kill.y >= k.y + 15.0 && kill.y <= k.bottom() && (kill.centre().x - k.centre().x).abs() <= 1.0, "{size:?} ch{ch} {band}: kill under the ring");
            }
            let high = Rect::of(&ui.get(&format!("mixer.ch{ch}.eq-high")));
            // Larger than before at desktop size (22 px): 24 px; the compact tier keeps 16 px.
            let min = if size.0 >= 1440 { 32.0 } else { 24.0 };
            assert!(high.w >= min, "{size:?}: EQ knob {}px", high.w);
        }
        let r = |l: &str| Rect::of(&ui.get(l));
        let (left, right) = (r("mixer.ch1.stem").x, r("mixer.ch2.cue").right());
        // The column is as wide as on main (two 50 px strips, 124 px with its
        // padding), so the decks keep their width: the CH FX selector spans
        // the column minus 4 px each side.
        let column = r("fx.channel-fx.type").w + 8.0;
        assert!(column <= 125.0, "{size:?}: mixer column {column} px");
        for ch in 1..=2 {
            let w = r(&format!("mixer.ch{ch}.cue")).right() - r(&format!("mixer.ch{ch}.stem")).x;
            assert!((w - 50.0).abs() <= 1.0, "{size:?}: strip {ch} is {w} px");
            let row = r(&format!("mixer.ch{ch}.eq-low.kill")).w.max(r(&format!("mixer.ch{ch}.eq-low")).w);
            assert!(row <= 51.0, "{size:?}: band row {row} px");
        }
        for key in ["mixer.swap.low", "mixer.swap.mid", "mixer.swap.high", "mixer.swap.at"] {
            let k = r(key);
            assert!(
                k.x >= left - 5.0 && k.right() <= right + 5.0,
                "{size:?}: {key} outside the strips ({} .. {})",
                k.x,
                k.right()
            );
        }
    }
    let ui = Ui::open((1440, 900));
    perform(&ui);
    ui.app.ui.invoke_set_deck_count(4);
    ui.pump(300);
    assert!(ui.wait_until(5.0, |u| u.has("mixer.ch4.eq-low.kill")));
}

/// Writes a library XML of `ui`'s library with a missing file and a stream
/// added; returns its path.
fn export_with_gaps(ui: &Ui) -> std::path::PathBuf {
    let xml = ui.fx.root.join("exchange/library.xml");
    std::fs::create_dir_all(xml.parent().unwrap()).unwrap();
    crate::import::export_xml(&ui.app, &xml);
    let gone = od_import::xml::location(&ui.fx.root.join("music/gone.wav"));
    let text = std::fs::read_to_string(&xml).unwrap().replace(
        "</COLLECTION>",
        &format!(
            r#"<TRACK TrackID="9001" Name="Lost Song" Artist="Nobody" Location="{gone}"/>
<TRACK TrackID="9002" Name="Radio" Artist="Stream" Location="https://example.invalid/live"/>
</COLLECTION>"#
        ),
    );
    std::fs::write(&xml, text).unwrap();
    xml
}

fn count_value(ui: &Ui, label: &str) -> String {
    ui.with_prefix("app.import-review.count")
        .into_iter()
        .map(|(_, v, _)| v)
        .find(|v| v.starts_with(&format!("{label} ")))
        .unwrap_or_else(|| panic!("count `{label}`"))
}

/// [54] Preview before anything is written, then before / after counts,
/// what did not transfer, and undo back to the library as it was.
#[test]
fn library_import_preview_result_and_undo() {
    let source = Ui::open((1440, 900));
    let xml = export_with_gaps(&source);
    let ui = Ui::open((1440, 900));
    let before = ui.fx.lib().counts().unwrap();

    // Preview: nothing written yet.
    crate::import::open_review(&ui.app, &xml, od_import::review::ExportFormat::LibraryXml);
    ui.pump(100);
    assert!(ui.value("app.import-review.title").contains("NOTHING IS WRITTEN"));
    assert_eq!(ui.value("app.import-review.file"), "library.xml");
    assert_eq!(count_value(&ui, "New tracks"), "New tracks  3");
    let left: Vec<String> = ui
        .with_prefix("app.import-review.left-out")
        .into_iter()
        .filter(|(l, _, _)| l == "app.import-review.left-out")
        .map(|(_, v, _)| v)
        .collect();
    assert_eq!(
        left,
        vec![
            "Nobody – Lost Song — file not found".to_string(),
            "Stream – Radio — not a file on this computer (stream or cloud)".to_string(),
        ]
    );
    assert_eq!(ui.fx.lib().counts().unwrap(), before, "the preview writes nothing");

    // Cancel leaves everything as it was.
    ui.click("app.import-review.cancel");
    ui.pump(100);
    assert!(!ui.has("app.import-review.title"));
    assert_eq!(ui.fx.lib().counts().unwrap(), before);

    // Import: before and after, and what did not transfer.
    crate::import::open_review(&ui.app, &xml, od_import::review::ExportFormat::LibraryXml);
    ui.pump(100);
    ui.click("app.import-review.run");
    assert!(
        ui.wait_until(60.0, |u| u.value("app.import-review.title") == "IMPORT DONE"),
        "the import finishes"
    );
    let after = ui.fx.lib().counts().unwrap();
    assert_eq!(after.tracks, before.tracks + 3);
    assert_eq!(
        count_value(&ui, "Tracks"),
        format!("Tracks {} {}", before.tracks, after.tracks)
    );
    assert!(ui.value("app.import-review.left-out-title").contains("DID NOT TRANSFER (2)"));
    let snapshot = ui.fx.library.parent().unwrap().join("import-undo.snapshot");
    assert!(snapshot.is_file(), "undo copy kept while the window is open");

    // Undo: the library as it was.
    ui.click("app.import-review.undo");
    assert!(ui.wait_until(10.0, |u| u.value("app.import-review.title") == "IMPORT UNDONE"));
    assert_eq!(ui.fx.lib().counts().unwrap(), before, "undo puts everything back");
    assert!(!snapshot.exists());
    ui.click("app.import-review.close");
    ui.pump(100);
    assert!(!ui.has("app.import-review.title"));
}
